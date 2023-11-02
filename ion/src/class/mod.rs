/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::collections::hash_map::Entry;
use std::ffi::CString;
use std::ptr;

use mozjs::glue::JS_GetReservedSlot;
use mozjs::jsapi::{
	Handle, JS_GetConstructor, JS_InitClass, JS_InstanceOf, JS_NewObjectWithGivenProto, JS_SetReservedSlot, JSFunction, JSFunctionSpec, JSObject,
	JSPropertySpec,
};
use mozjs::jsval::{PrivateValue, UndefinedValue};

use crate::{Arguments, Context, Function, Local, Object};
pub use crate::class::native::{MAX_PROTO_CHAIN_LENGTH, NativeClass, PrototypeChain, TypeIdWrapper};
pub use crate::class::reflect::{Castable, DerivedFrom, NativeObject, Reflector};
use crate::functions::NativeFunction;

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
	const NAME: &'static str;

	fn class() -> &'static NativeClass;

	fn parent_class_info(_: &Context) -> Option<(&'static NativeClass, Local<*mut JSObject>)> {
		None
	}

	fn constructor() -> (NativeFunction, u32);

	fn functions() -> &'static [JSFunctionSpec] {
		&[JSFunctionSpec::ZERO]
	}

	fn properties() -> &'static [JSPropertySpec] {
		&[JSPropertySpec::ZERO]
	}

	fn static_functions() -> &'static [JSFunctionSpec] {
		&[JSFunctionSpec::ZERO]
	}

	fn static_properties() -> &'static [JSPropertySpec] {
		&[JSPropertySpec::ZERO]
	}

	fn init_class<'cx>(cx: &'cx Context, object: &mut Object) -> (bool, &'cx ClassInfo) {
		let infos = unsafe { &mut (*cx.get_inner_data().as_ptr()).class_infos };
		let entry = infos.entry(TypeId::of::<Self>());

		match entry {
			Entry::Occupied(o) => (false, o.into_mut()),
			Entry::Vacant(entry) => {
				let (parent_class, parent_proto) = Self::parent_class_info(cx)
					.map(|(class, proto)| (&class.base as *const _, Object::from(proto)))
					.unwrap_or_else(|| (ptr::null(), Object::new(cx)));
				let (constructor, nargs) = Self::constructor();
				let properties = Self::properties();
				let functions = Self::functions();
				let static_properties = Self::static_properties();
				let static_functions = Self::static_functions();

				let name = CString::new(Self::NAME).unwrap();

				let class = unsafe {
					JS_InitClass(
						cx.as_ptr(),
						object.handle().into(),
						parent_class,
						parent_proto.handle().into(),
						name.as_ptr().cast(),
						Some(constructor),
						nargs,
						properties.as_ptr(),
						functions.as_ptr(),
						static_properties.as_ptr(),
						static_functions.as_ptr(),
					)
				};
				let prototype = cx.root_object(class);

				let constructor = Object::from(cx.root_object(unsafe { JS_GetConstructor(cx.as_ptr(), prototype.handle().into()) }));
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
		unsafe { JS_NewObjectWithGivenProto(cx.as_ptr(), &Self::class().base, Handle::from_marked_location(&info.prototype)) }
	}

	fn new_object(cx: &Context, native: Box<Self>) -> *mut JSObject {
		let object = Self::new_raw_object(cx);
		unsafe {
			Self::set_private(object, native);
		}
		object
	}

	fn get_private<'a>(object: &Object<'a>) -> &'a Self {
		unsafe {
			let mut value = UndefinedValue();
			JS_GetReservedSlot(object.handle().get(), 0, &mut value);
			&*(value.to_private() as *const Self)
		}
	}

	fn get_mut_private<'a>(object: &mut Object<'a>) -> &'a mut Self {
		unsafe {
			let mut value = UndefinedValue();
			JS_GetReservedSlot(object.handle().get(), 0, &mut value);
			&mut *(value.to_private() as *mut Self)
		}
	}

	unsafe fn set_private(object: *mut JSObject, native: Box<Self>) {
		native.reflector().set(object);
		unsafe {
			JS_SetReservedSlot(object, 0, &PrivateValue(Box::into_raw(native).cast_const().cast()));
		}
	}

	fn instance_of(cx: &Context, object: &Object, args: Option<&Arguments>) -> bool {
		unsafe {
			let args = args.map(|a| a.call_args()).as_mut().map_or(ptr::null_mut(), |args| args);
			JS_InstanceOf(cx.as_ptr(), object.handle().into(), &Self::class().base, args)
		}
	}
}
