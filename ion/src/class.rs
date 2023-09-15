/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::ffi::c_void;
use std::ptr;

use mozjs::gc::Traceable;
use mozjs::glue::JS_GetReservedSlot;
use mozjs::jsapi::{
	Heap, JS_GetConstructor, JS_InitClass, JS_InstanceOf, JS_NewObjectWithGivenProto, JS_SetReservedSlot, JSClass, JSFunction, JSFunctionSpec,
	JSObject, JSPropertySpec, JSTracer,
};
use mozjs::jsval::{PrivateValue, UndefinedValue};

use crate::{Arguments, Context, Error, ErrorKind, Function, Local, Object, Result, Value};
use crate::conversions::FromValue;
use crate::functions::NativeFunction;

/// Stores information about a native class created for JS.
#[derive(Debug)]
pub struct ClassInfo {
	#[allow(dead_code)]
	constructor: Box<Heap<*mut JSFunction>>,
	prototype: Box<Heap<*mut JSObject>>,
}

unsafe impl Traceable for ClassInfo {
	unsafe fn trace(&self, trc: *mut JSTracer) {
		self.constructor.trace(trc);
		self.prototype.trace(trc);
	}
}

pub trait ClassDefinition {
	const NAME: &'static str;
	const PARENT_PROTOTYPE_CHAIN_LENGTH: u32 = 0;

	fn class() -> &'static JSClass;

	fn parent_prototype<'cx>(_: &'cx Context) -> Option<Local<'cx, *mut JSObject>> {
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
		let infos = unsafe { &mut (*cx.get_inner_data()).class_infos };
		let info = infos.get(&TypeId::of::<Self>());

		if let Some(info) = info {
			unsafe {
				return (false, &*(info as *const _));
			}
		}

		let class = Self::class();
		let parent_proto = Self::parent_prototype(cx).map(Object::from).unwrap_or_else(|| Object::new(cx));
		let (constructor, nargs) = Self::constructor();
		let properties = Self::properties();
		let functions = Self::functions();
		let static_properties = Self::static_properties();
		let static_functions = Self::static_functions();

		let class = unsafe {
			JS_InitClass(
				cx.as_ptr(),
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

		let constructor = Object::from(cx.root_object(unsafe { JS_GetConstructor(cx.as_ptr(), class.handle().into()) }));
		let constructor = Function::from_object(cx, &constructor).unwrap();

		let info = ClassInfo {
			constructor: Heap::boxed(constructor.get()),
			prototype: Heap::boxed(class.get()),
		};

		let info = infos.entry(TypeId::of::<Self>()).or_insert(info);
		(true, info)
	}

	fn new_object(cx: &Context, native: Self) -> *mut JSObject
	where
		Self: Sized + 'static,
	{
		let infos = unsafe { &mut (*cx.get_inner_data()).class_infos };
		let info = infos.get(&TypeId::of::<Self>()).expect("Uninitialised Class");
		let boxed = Box::new(Some(native));
		unsafe {
			let obj = JS_NewObjectWithGivenProto(cx.as_ptr(), Self::class(), info.prototype.handle());
			JS_SetReservedSlot(
				obj,
				Self::PARENT_PROTOTYPE_CHAIN_LENGTH,
				&PrivateValue(Box::into_raw(boxed) as *mut c_void),
			);
			obj
		}
	}

	#[allow(clippy::mut_from_ref)]
	fn get_private<'a>(object: &'a Object) -> &'a mut Self
	where
		Self: Sized,
	{
		unsafe {
			let mut value = UndefinedValue();
			JS_GetReservedSlot(object.handle().get(), Self::PARENT_PROTOTYPE_CHAIN_LENGTH, &mut value);
			(*(value.to_private() as *mut Option<Self>)).as_mut().unwrap()
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
pub unsafe fn class_from_value<'cx: 'v, 'v, T: ClassDefinition + Clone>(cx: &'cx Context, value: &Value<'v>) -> Result<T> {
	let object = Object::from_value(cx, value, true, ()).unwrap();
	if T::instance_of(cx, &object, None) {
		Ok(T::get_private(&object).clone())
	} else {
		Err(Error::new(&format!("Expected {}", T::NAME), ErrorKind::Type))
	}
}
