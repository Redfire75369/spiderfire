/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::collections::hash_map::Entry;
use std::ffi::CStr;
use std::ptr;

use mozjs::gc::HandleObject;
use mozjs::gc::Traceable;
use mozjs::glue::JS_GetReservedSlot;
use mozjs::jsapi::{
	GCContext, Handle, JSFunction, JSFunctionSpec, JSObject, JSPropertySpec, JSTracer, JS_GetConstructor,
	JS_HasInstance, JS_InitClass, JS_InstanceOf, JS_NewObjectWithGivenProto, JS_SetReservedSlot,
};
use mozjs::jsval::{NullValue, PrivateValue, UndefinedValue};

pub use crate::class::native::{NativeClass, PrototypeChain, TypeIdWrapper, MAX_PROTO_CHAIN_LENGTH};
pub use crate::class::reflect::{Castable, DerivedFrom, NativeObject, Reflector};
use crate::conversions::ToValue;
use crate::function::NativeFunction;
use crate::{Context, Error, ErrorKind, Function, Local, Object, Result};

mod native;
mod reflect;

/// Stores information about a native class created for JS.
#[allow(dead_code)]
#[derive(Debug)]
pub struct ClassInfo {
	class: &'static NativeClass,
	constructor: *mut JSFunction,
	prototype: *mut JSObject,
}

pub trait ClassDefinition: NativeObject {
	fn class() -> &'static NativeClass;

	fn proto_class() -> Option<&'static NativeClass> {
		None
	}

	fn parent_prototype(_: &Context) -> Option<Local<*mut JSObject>> {
		None
	}

	fn constructor() -> (NativeFunction, u32);

	fn functions() -> Option<&'static [JSFunctionSpec]> {
		None
	}

	fn properties() -> Option<&'static [JSPropertySpec]> {
		None
	}

	fn static_functions() -> Option<&'static [JSFunctionSpec]> {
		None
	}

	fn static_properties() -> Option<&'static [JSPropertySpec]> {
		None
	}

	fn init_class<'cx>(cx: &'cx Context, object: &Object) -> (bool, &'cx ClassInfo) {
		let infos = unsafe { &mut (*cx.get_inner_data().as_ptr()).class_infos };

		match infos.entry(TypeId::of::<Self>()) {
			Entry::Occupied(o) => (false, o.into_mut()),
			Entry::Vacant(entry) => {
				let proto_class = Self::proto_class().map_or_else(ptr::null, |class| &class.base);
				let parent_proto = Self::parent_prototype(cx).map_or_else(HandleObject::null, |proto| proto.handle());

				let (constructor, nargs) = Self::constructor();

				let properties = Self::properties();
				let functions = Self::functions();
				let static_properties = Self::static_properties();
				let static_functions = Self::static_functions();

				assert!(has_zero_spec(properties));
				assert!(has_zero_spec(functions));
				assert!(has_zero_spec(static_properties));
				assert!(has_zero_spec(static_functions));

				let class = unsafe {
					JS_InitClass(
						cx.as_ptr(),
						object.handle().into(),
						proto_class,
						parent_proto.into(),
						Self::class().base.name,
						Some(constructor),
						nargs,
						unwrap_specs(properties),
						unwrap_specs(functions),
						unwrap_specs(static_properties),
						unwrap_specs(static_functions),
					)
				};
				let prototype = cx.root(class);

				let constructor = unsafe { JS_GetConstructor(cx.as_ptr(), prototype.handle().into()) };
				let constructor = Object::from(cx.root(constructor));
				let constructor = Function::from_object(cx, &constructor).unwrap();

				let class_info = ClassInfo {
					class: Self::class(),
					constructor: constructor.get(),
					prototype: prototype.get(),
				};

				(true, entry.insert(class_info))
			}
		}
	}

	fn new_raw_object(cx: &Context) -> *mut JSObject {
		let infos = unsafe { &mut (*cx.get_inner_data().as_ptr()).class_infos };
		let info = infos.get(&TypeId::of::<Self>()).expect("Uninitialised Class");
		unsafe {
			JS_NewObjectWithGivenProto(
				cx.as_ptr(),
				&Self::class().base,
				Handle::from_marked_location(&info.prototype),
			)
		}
	}

	fn new_object(cx: &Context, native: Box<Self>) -> *mut JSObject {
		let object = Self::new_raw_object(cx);
		unsafe {
			Self::set_private(object, native);
		}
		object
	}

	unsafe fn get_private_unchecked<'a>(object: &Object<'a>) -> &'a Self {
		unsafe {
			let mut value = UndefinedValue();
			JS_GetReservedSlot(object.handle().get(), 0, &mut value);
			&*(value.to_private().cast::<Self>())
		}
	}

	fn get_private<'a>(cx: &Context, object: &Object<'a>) -> Result<&'a Self> {
		if Self::instance_of(cx, object) || Self::has_instance(cx, object)? {
			Ok(unsafe { Self::get_private_unchecked(object) })
		} else {
			Err(private_error(Self::class()))
		}
	}

	#[allow(clippy::mut_from_ref)]
	unsafe fn get_mut_private_unchecked<'a>(object: &Object<'a>) -> &'a mut Self {
		unsafe {
			let mut value = UndefinedValue();
			JS_GetReservedSlot(object.handle().get(), 0, &mut value);
			&mut *(value.to_private().cast_mut().cast::<Self>())
		}
	}

	#[allow(clippy::mut_from_ref)]
	fn get_mut_private<'a>(cx: &Context, object: &Object<'a>) -> Result<&'a mut Self> {
		if Self::instance_of(cx, object) || Self::has_instance(cx, object)? {
			Ok(unsafe { Self::get_mut_private_unchecked(object) })
		} else {
			Err(private_error(Self::class()))
		}
	}

	unsafe fn set_private(object: *mut JSObject, native: Box<Self>) {
		native.reflector().set(object);
		unsafe {
			JS_SetReservedSlot(object, 0, &PrivateValue(Box::into_raw(native).cast_const().cast()));
		}
	}

	fn instance_of(cx: &Context, object: &Object) -> bool {
		unsafe {
			JS_InstanceOf(
				cx.as_ptr(),
				object.handle().into(),
				&Self::class().base,
				ptr::null_mut(),
			)
		}
	}

	fn has_instance(cx: &Context, object: &Object) -> Result<bool> {
		let infos = unsafe { &mut (*cx.get_inner_data().as_ptr()).class_infos };
		let constructor =
			Function::from(cx.root(infos.get(&TypeId::of::<Self>()).expect("Uninitialised Class").constructor))
				.to_object(cx);
		let object = object.as_value(cx);
		let mut has_instance = false;
		let result = unsafe {
			JS_HasInstance(
				cx.as_ptr(),
				constructor.handle().into(),
				object.handle().into(),
				&mut has_instance,
			)
		};
		if result {
			Ok(has_instance)
		} else {
			Err(Error::none())
		}
	}
}

trait SpecZero {
	fn is_zeroed(&self) -> bool;
}

impl SpecZero for JSFunctionSpec {
	fn is_zeroed(&self) -> bool {
		self.is_zeroed()
	}
}

impl SpecZero for JSPropertySpec {
	fn is_zeroed(&self) -> bool {
		self.is_zeroed()
	}
}

fn has_zero_spec<T: SpecZero>(specs: Option<&[T]>) -> bool {
	specs.and_then(|s| s.last()).map_or(true, |specs| specs.is_zeroed())
}

fn unwrap_specs<T>(specs: Option<&[T]>) -> *const T {
	specs.map_or_else(ptr::null, |specs| specs.as_ptr())
}

fn private_error(class: &'static NativeClass) -> Error {
	let name = unsafe { CStr::from_ptr(class.base.name).to_str().unwrap() };
	Error::new(format!("Object does not implement interface {}", name), ErrorKind::Type)
}

#[doc(hidden)]
pub unsafe extern "C" fn finalise_native_object_operation<T>(_: *mut GCContext, this: *mut JSObject) {
	let mut value = NullValue();
	unsafe {
		JS_GetReservedSlot(this, 0, &mut value);
	}
	if value.is_double() && value.asBits_ & 0xFFFF000000000000 == 0 {
		let private = value.to_private().cast_mut().cast::<T>();
		let _ = unsafe { Box::from_raw(private) };
	}
}

#[doc(hidden)]
pub unsafe extern "C" fn trace_native_object_operation<T: Traceable>(trc: *mut JSTracer, this: *mut JSObject) {
	let mut value = NullValue();
	unsafe {
		JS_GetReservedSlot(this, 0, &mut value);
	}
	if value.is_double() && value.asBits_ & 0xFFFF000000000000 == 0 {
		unsafe {
			let private = &*(value.to_private().cast::<T>());
			private.trace(trc);
		}
	}
}
