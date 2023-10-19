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
	Handle, JS_GetConstructor, JS_InitClass, JS_InstanceOf, JS_NewObjectWithGivenProto, JS_SetReservedSlot, JSClass, JSFunction, JSFunctionSpec,
	JSObject, JSPropertySpec,
};
use mozjs::jsval::{PrivateValue, UndefinedValue};

use crate::{Arguments, Context, Error, ErrorKind, Function, Local, Object, Result, Value};
use crate::conversions::FromValue;
use crate::functions::NativeFunction;

/// Stores information about a native class created for JS.
#[allow(dead_code)]
#[derive(Debug)]
pub struct ClassInfo {
	class: &'static JSClass,
	constructor: *mut JSFunction,
	prototype: *mut JSObject,
}

pub trait ClassDefinition {
	const NAME: &'static str;
	const PARENT_PROTOTYPE_CHAIN_LENGTH: u32 = 0;

	fn class() -> &'static JSClass;

	fn parent_class_info<'cx>(_: &'cx Context) -> Option<(&'static JSClass, Local<'cx, *mut JSObject>)> {
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

	fn init_class<'cx>(cx: &'cx Context, object: &mut Object) -> (bool, &'cx ClassInfo)
	where
		Self: Sized + 'static,
	{
		let infos = unsafe { &mut (*cx.get_inner_data().as_ptr()).class_infos };
		let entry: Entry<'cx, _, _> = infos.entry(TypeId::of::<Self>());

		match entry {
			Entry::Occupied(o) => (false, o.into_mut()),
			Entry::Vacant(entry) => {
				let (parent_class, parent_proto) = Self::parent_class_info(cx)
					.map(|(class, proto)| (class as *const _, Object::from(proto)))
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

	fn new_raw_object(cx: &Context) -> *mut JSObject
	where
		Self: Sized + 'static,
	{
		let infos = unsafe { &mut (*cx.get_inner_data().as_ptr()).class_infos };
		let info = infos.get(&TypeId::of::<Self>()).expect("Uninitialised Class");
		unsafe { JS_NewObjectWithGivenProto(cx.as_ptr(), Self::class(), Handle::from_marked_location(&info.prototype)) }
	}

	fn new_object(cx: &Context, native: Self) -> *mut JSObject
	where
		Self: Sized + 'static,
	{
		let object = Self::new_raw_object(cx);
		let boxed = Box::new(Some(native));
		unsafe {
			JS_SetReservedSlot(object, Self::PARENT_PROTOTYPE_CHAIN_LENGTH, &PrivateValue(Box::into_raw(boxed).cast()));
		}
		object
	}

	#[allow(clippy::mut_from_ref)]
	fn get_private<'a>(object: &'a Object) -> &'a Self
	where
		Self: Sized,
	{
		unsafe {
			let mut value = UndefinedValue();
			JS_GetReservedSlot(object.handle().get(), Self::PARENT_PROTOTYPE_CHAIN_LENGTH, &mut value);
			(*(value.to_private() as *const Option<Self>)).as_ref().unwrap()
		}
	}

	fn get_mut_private<'a>(object: &'a mut Object) -> &'a mut Self
	where
		Self: Sized,
	{
		unsafe {
			let mut value = UndefinedValue();
			JS_GetReservedSlot(object.handle().get(), Self::PARENT_PROTOTYPE_CHAIN_LENGTH, &mut value);
			(*(value.to_private() as *mut Option<Self>)).as_mut().unwrap()
		}
	}

	fn set_private(object: *mut JSObject, native: Self)
	where
		Self: Sized,
	{
		let boxed = Box::new(Some(native));
		unsafe {
			JS_SetReservedSlot(object, Self::PARENT_PROTOTYPE_CHAIN_LENGTH, &PrivateValue(Box::into_raw(boxed).cast()));
		}
	}

	fn instance_of(cx: &Context, object: &Object, args: Option<&Arguments>) -> bool {
		unsafe {
			let args = args.map(|a| a.call_args()).as_mut().map_or(ptr::null_mut(), |args| args);
			JS_InstanceOf(cx.as_ptr(), object.handle().into(), Self::class(), args)
		}
	}
}

/// Converts an instance of a native class into its native value, by cloning it.
pub fn class_from_value<'cx: 'v, 'v, T: ClassDefinition + Clone>(cx: &'cx Context, value: &Value<'v>) -> Result<T> {
	let object = Object::from_value(cx, value, true, ()).unwrap();
	if T::instance_of(cx, &object, None) {
		Ok(T::get_private(&object).clone())
	} else {
		Err(Error::new(&format!("Expected {}", T::NAME), ErrorKind::Type))
	}
}
