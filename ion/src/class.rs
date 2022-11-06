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
	Handle, JS_GetConstructor, JS_GetInstancePrivate, JS_InitClass, JS_InstanceOf, JS_NewObjectWithGivenProto, JSClass, JSFunction, JSFunctionSpec,
	JSObject, JSPropertySpec, SetPrivate,
};

use crate::{Arguments, Context, Error, ErrorKind, Function, Object, Result, Value};
use crate::conversions::FromValue;
use crate::functions::NativeFunction;

// TODO: Move into Context Wrapper
thread_local!(pub static CLASS_INFOS: RefCell<HashMap<TypeId, ClassInfo>> = RefCell::new(HashMap::new()));

#[derive(Copy, Clone, Debug)]
pub struct ClassInfo {
	#[allow(dead_code)]
	constructor: *mut JSFunction,
	prototype: *mut JSObject,
}

pub trait ClassInitialiser {
	const NAME: &'static str;

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
			let b = Box::new(native);
			unsafe {
				let obj = JS_NewObjectWithGivenProto(**cx, Self::class(), Handle::from_marked_location(&info.prototype));
				SetPrivate(obj, Box::into_raw(b) as *mut c_void);
				obj
			}
		})
	}

	fn get_private<'a>(cx: &Context, object: &Object, args: Option<&Arguments>) -> Result<&'a mut Self>
	where
		Self: Sized,
	{
		unsafe {
			let args = args.map(|a| a.call_args()).as_mut().map_or(ptr::null_mut(), |args| args);
			let ptr = JS_GetInstancePrivate(**cx, object.handle().into(), Self::class(), args) as *mut Self;
			if !ptr.is_null() {
				Ok(&mut *ptr)
			} else {
				Err(Error::new(
					&format!("Could not get private value in {}. It may have been destroyed.", Self::NAME),
					None,
				))
			}
		}
	}

	fn take_private(cx: &Context, object: &Object, args: Option<&Arguments>) -> Result<Box<Self>>
	where
		Self: Sized,
	{
		unsafe {
			let args = args.map(|a| a.call_args()).as_mut().map_or(ptr::null_mut(), |args| args);
			let ptr = JS_GetInstancePrivate(**cx, object.handle().into(), Self::class(), args) as *mut Self;
			if !ptr.is_null() {
				let private = Box::from_raw(ptr);
				SetPrivate(object.handle().get(), ptr::null_mut() as *mut c_void);
				Ok(private)
			} else {
				Err(Error::new(
					&format!("Could not get private value in {}. It may have been destroyed.", Self::NAME),
					None,
				))
			}
		}
	}

	fn instance_of(cx: &Context, object: &Object, args: Option<&Arguments>) -> bool {
		unsafe {
			let args = args.map(|a| a.call_args()).as_mut().map_or(ptr::null_mut(), |args| args);
			JS_InstanceOf(**cx, object.handle().into(), Self::class(), args)
		}
	}
}

pub unsafe fn class_from_value<'cx: 'v, 'v, T: ClassInitialiser + Clone>(cx: &'cx Context, value: &Value<'v>) -> Result<T> {
	let object = Object::from_value(cx, value, true, ()).unwrap();
	if T::instance_of(cx, &object, None) {
		T::get_private(cx, &object, None).map(|c| c.clone())
	} else {
		Err(Error::new(&format!("Expected {}", T::NAME), ErrorKind::Type))
	}
}
