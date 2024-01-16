/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::collections::hash_map::Entry;
use std::ffi::CStr;
use std::ptr;

use mozjs::glue::JS_GetReservedSlot;
use mozjs::jsapi::{
	Handle, JS_GetConstructor, JS_InitClass, JS_InstanceOf, JS_NewObjectWithGivenProto, JS_SetReservedSlot, JSFunction,
	JSFunctionSpec, JSObject, JSPropertySpec,
};
use mozjs::jsval::{PrivateValue, UndefinedValue};
use mozjs::rust::HandleObject;

use crate::{Context, Error, ErrorKind, Function, Local, Object, Result};
pub use crate::class::native::{MAX_PROTO_CHAIN_LENGTH, NativeClass, PrototypeChain, TypeIdWrapper};
pub use crate::class::reflect::{Castable, DerivedFrom, NativeObject, Reflector};
use crate::function::NativeFunction;

mod native;
mod reflect;

/// Stores information about a native class created for JS.
#[allow(dead_code)]
#[derive(Debug)]
pub struct ClassInfo {
	pub class: &'static NativeClass,
	pub constructor: *mut JSFunction,
	pub prototype: *mut JSObject,
}

pub trait ClassDefinition: NativeObject {
	fn class() -> &'static NativeClass;

	fn parent_class_info(_: &Context) -> Option<(&'static NativeClass, Local<*mut JSObject>)> {
		None
	}

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
				let proto_class = Self::proto_class().map_or_else(ptr::null, |class| &class.base as *const _);
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
				let prototype = cx.root_object(class);

				let constructor = unsafe { JS_GetConstructor(cx.as_ptr(), prototype.handle().into()) };
				let constructor = Object::from(cx.root_object(constructor));
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
			&*(value.to_private() as *const Self)
		}
	}

	fn get_private<'a>(cx: &Context, object: &Object<'a>) -> Result<&'a Self> {
		if Self::instance_of(cx, object) {
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
			&mut *(value.to_private() as *mut Self)
		}
	}

	#[allow(clippy::mut_from_ref)]
	fn get_mut_private<'a>(cx: &Context, object: &Object<'a>) -> Result<&'a mut Self> {
		if Self::instance_of(cx, object) {
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
	Error::new(
		&format!("Object does not implement interface {}", name),
		ErrorKind::Type,
	)
}
