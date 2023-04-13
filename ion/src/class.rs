/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;

use mozjs::jsapi::{
	Handle, JS_GetConstructor, JS_InitClass, JS_InstanceOf, JS_NewObjectWithGivenProto, JSClass, JSFunction, JSFunctionSpec, JSObject,
	JSPropertySpec, JS_SetReservedSlot,
};
use mozjs::glue::JS_GetReservedSlot;
use mozjs::jsval::PrivateValue;
use mozjs_sys::jsval::UndefinedValue;

use crate::{Arguments, Context, Error, ErrorKind, Function, Object, Result, Value};
use crate::conversions::FromValue;
use crate::functions::NativeFunction;

// TODO: Move into Context Wrapper
thread_local!(pub static CLASS_INFOS: RefCell<HashMap<TypeId, ClassInfo>> = RefCell::new(HashMap::new()));

/// Stores information about a native class created for JS.
#[derive(Copy, Clone, Debug)]
pub struct ClassInfo {
	#[allow(dead_code)]
	constructor: *mut JSFunction,
	prototype: *mut JSObject,
}

pub trait ClassInitialiser {
	const NAME: &'static str;
	const PARENT_PROTOTYPE_CHAIN_LENGTH: u32;

	fn class() -> &'static JSClass;

	fn parent_info(_: &Context) -> Option<ClassInfo> {
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

	fn init_class(cx: &Context, object: &mut Object) -> (bool, ClassInfo)
	where
		Self: Sized + 'static,
	{
		let class_info = CLASS_INFOS.with(|infos| infos.borrow_mut().get(&TypeId::of::<Self>()).cloned());

		if let Some(class_info) = class_info {
			return (false, class_info);
		}

		let class = Self::class();
		let parent_proto = Self::parent_info(cx)
			.map(|ci| cx.root_object(ci.prototype).into())
			.unwrap_or_else(|| Object::new(cx));
		let (constructor, nargs) = Self::constructor();
		let properties = Self::properties();
		let functions = Self::functions();
		let static_properties = Self::static_properties();
		let static_functions = Self::static_functions();

		let class = unsafe {
			JS_InitClass(
				**cx,
				object.handle().into(),
				parent_proto.handle().into(),
				class,
				Some(constructor),
				nargs,
				properties.as_ptr() as *const _,
				functions.as_ptr() as *const _,
				static_properties.as_ptr() as *const _,
				static_functions.as_ptr() as *const _,
			)
		};
		let class = cx.root_object(class);

		let constructor = Object::from(cx.root_object(unsafe { JS_GetConstructor(**cx, class.handle().into()) }));
		let constructor = Function::from_object(cx, &constructor).unwrap();

		let class_info = ClassInfo {
			constructor: **constructor,
			prototype: *class,
		};

		CLASS_INFOS.with(|infos| {
			let mut infos = infos.borrow_mut();
			(*infos).insert(TypeId::of::<Self>(), class_info);
			(true, class_info)
		})
	}

	fn new_object(cx: &Context, native: Self) -> *mut JSObject
	where
		Self: Sized + 'static,
	{
		CLASS_INFOS.with(|infos| {
			let infos = infos.borrow();
			let info = (*infos).get(&TypeId::of::<Self>()).expect("Uninitialised Class");
			let b = Box::new(Some(native));
			unsafe {
				let obj = JS_NewObjectWithGivenProto(**cx, Self::class(), Handle::from_marked_location(&info.prototype));
				JS_SetReservedSlot(obj, Self::PARENT_PROTOTYPE_CHAIN_LENGTH, &PrivateValue(Box::into_raw(b) as *mut c_void));
				obj
			}
		})
	}

	fn get_private<'a>(object: &'a Object) -> &'a mut Self
	where
		Self: Sized,
	{
		unsafe {
			let mut value = UndefinedValue();
			JS_GetReservedSlot(***object, Self::PARENT_PROTOTYPE_CHAIN_LENGTH, &mut value);
			(&mut *(value.to_private() as *mut Option<Self>)).as_mut().unwrap()
		}
	}

	fn instance_of(cx: &Context, object: &Object, args: Option<&Arguments>) -> bool {
		unsafe {
			let args = args.map(|a| a.call_args()).as_mut().map_or(ptr::null_mut(), |args| args);
			JS_InstanceOf(**cx, object.handle().into(), Self::class(), args)
		}
	}
}

/// Converts an instance of a native class into its native value, by cloning it.
pub unsafe fn class_from_value<'cx: 'v, 'v, T: ClassInitialiser + Clone>(cx: &'cx Context, value: &Value<'v>) -> Result<T> {
	let object = Object::from_value(cx, value, true, ()).unwrap();
	if T::instance_of(cx, &object, None) {
		Ok(T::get_private(&object).clone())
	} else {
		Err(Error::new(&format!("Expected {}", T::NAME), ErrorKind::Type))
	}
}
