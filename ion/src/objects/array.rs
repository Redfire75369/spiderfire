/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use mozjs::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use mozjs::error::throw_type_error;
use mozjs::jsapi::{HandleValueArray, JSTracer, Value};
use mozjs::jsapi::{
	AssertSameCompartment, GetArrayLength, IsArray, JS_DefineElement, JS_DeleteElement1, JS_GetElement, JS_HasElement, JS_SetElement, NewArrayObject,
};
use mozjs::jsval::{ObjectValue, UndefinedValue};
use mozjs::rust::{CustomTrace, GCMethods, HandleValue, maybe_wrap_object_value, MutableHandleValue};
use mozjs_sys::jsgc::RootKind;

use crate::exception::Exception;
use crate::IonContext;
use crate::objects::object::IonRawObject;
use crate::types::values::from_value;

#[derive(Clone, Copy, Debug)]
pub struct IonArray {
	obj: IonRawObject,
}

impl IonArray {
	/// Returns the wrapped [IonRawObject].
	pub unsafe fn raw(&self) -> IonRawObject {
		self.obj
	}

	/// Creates an empty [IonArray].
	#[allow(dead_code)]
	unsafe fn new(cx: IonContext) -> IonArray {
		IonArray::from_slice(cx, &[])
	}

	/// Creates an [IonArray] from a [Vec].
	pub unsafe fn from_vec(cx: IonContext, vec: Vec<Value>) -> IonArray {
		IonArray::from_slice(cx, vec.as_slice())
	}

	/// Creates an [IonArray] from a slice.
	pub unsafe fn from_slice(cx: IonContext, slice: &[Value]) -> IonArray {
		IonArray::from_handle(cx, HandleValueArray::from_rooted_slice(slice))
	}

	/// Creates an [IonArray] from a [HandleValueArray].
	pub unsafe fn from_handle(cx: IonContext, handle: HandleValueArray) -> IonArray {
		IonArray::from(cx, NewArrayObject(cx, &handle)).unwrap()
	}

	/// Creates an [IonArray] from an [IonRawObject].
	///
	/// Returns [None] if the object is not an array.
	pub unsafe fn from(cx: IonContext, obj: IonRawObject) -> Option<IonArray> {
		if IonArray::is_array_raw(cx, obj) {
			Some(IonArray { obj })
		} else {
			throw_type_error(cx, "Object cannot be converted to Array");
			None
		}
	}

	/// Creates an [IonArray] from a [Value].
	///
	/// Returns [None] if the value is not an object or an array.
	pub unsafe fn from_value(cx: IonContext, val: Value) -> Option<IonArray> {
		if val.is_object() {
			IonArray::from(cx, val.to_object())
		} else {
			None
		}
	}

	/// Converts an [IonArray] to a [Vec].
	///
	/// Returns an empty [Vec] if the conversion fails.
	pub unsafe fn to_vec(&self, cx: IonContext) -> Vec<Value> {
		rooted!(in(cx) let obj = ObjectValue(self.obj));
		if let ConversionResult::Success(vec) = Vec::<Value>::from_jsval(cx, obj.handle(), ()).unwrap() {
			vec
		} else {
			Vec::new()
		}
	}

	/// Converts an [IonArray] to a [Value].
	pub unsafe fn to_value(&self) -> Value {
		ObjectValue(self.obj)
	}

	/// Returns the length of an [IonArray].
	pub unsafe fn len(&self, cx: IonContext) -> u32 {
		rooted!(in(cx) let robj = self.obj);
		let mut length = 0;
		GetArrayLength(cx, robj.handle().into(), &mut length);
		length
	}

	/// Checks if an array has the given index.
	pub unsafe fn has(&self, cx: IonContext, index: u32) -> bool {
		rooted!(in(cx) let robj = self.obj);
		let mut found = false;
		if JS_HasElement(cx, robj.handle().into(), index, &mut found) {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Gets the [Value] at the given index.
	pub unsafe fn get(&self, cx: IonContext, index: u32) -> Option<Value> {
		if self.has(cx, index) {
			rooted!(in(cx) let robj = self.obj);
			rooted!(in(cx) let mut rval: Value);
			JS_GetElement(cx, robj.handle().into(), index, rval.handle_mut().into());
			Some(rval.get())
		} else {
			None
		}
	}

	/// Gets the [Value] at the given index as a Rust type.
	pub unsafe fn get_as<T: FromJSValConvertible>(&self, cx: IonContext, index: u32, config: T::Config) -> Option<T> {
		let opt = self.get(cx, index);
		if let Some(val) = opt {
			from_value(cx, val, config)
		} else {
			None
		}
	}

	/// Sets the [Value] at the given index.
	pub unsafe fn set(&mut self, cx: IonContext, index: u32, value: Value) -> bool {
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let rval = value);
		JS_SetElement(cx, robj.handle().into(), index, rval.handle().into())
	}

	pub unsafe fn set_as<T: ToJSValConvertible + RootKind + GCMethods>(&mut self, cx: IonContext, index: u32, value: T) -> bool {
		rooted!(in(cx) let mut val = UndefinedValue());
		value.to_jsval(cx, val.handle_mut());
		self.set(cx, index, val.get())
	}

	/// Defines the [Value] at the given index with the given attributes.
	unsafe fn define(&mut self, cx: IonContext, index: u32, value: Value, attrs: u32) -> bool {
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let rval = value);
		JS_DefineElement(cx, robj.handle().into(), index, rval.handle().into(), attrs)
	}

	pub unsafe fn define_as<T: ToJSValConvertible + RootKind + GCMethods>(&mut self, cx: IonContext, index: u32, value: T, attrs: u32) -> bool {
		rooted!(in(cx) let mut val = UndefinedValue());
		value.to_jsval(cx, val.handle_mut());
		self.define(cx, index, val.get(), attrs)
	}

	/// Deletes the [Value] at the given index.
	#[allow(dead_code)]
	unsafe fn delete(&mut self, cx: IonContext, index: u32) -> bool {
		rooted!(in(cx) let robj = self.obj);
		JS_DeleteElement1(cx, robj.handle().into(), index)
	}

	/// Checks if an [IonRawObject] is an array.
	pub unsafe fn is_array_raw(cx: IonContext, obj: IonRawObject) -> bool {
		rooted!(in(cx) let mut robj = obj);
		let mut is_array = false;
		IsArray(cx, robj.handle().into(), &mut is_array) && is_array
	}

	pub unsafe fn is_array(&self, cx: IonContext) -> bool {
		IonArray::is_array_raw(cx, self.obj)
	}
}

impl FromJSValConvertible for IonArray {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: IonContext, value: HandleValue, _option: ()) -> Result<ConversionResult<IonArray>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "Value is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		if let Some(array) = IonArray::from(cx, value.to_object()) {
			Ok(ConversionResult::Success(array))
		} else {
			Err(())
		}
	}
}

impl ToJSValConvertible for IonArray {
	#[inline]
	unsafe fn to_jsval(&self, cx: IonContext, mut rval: MutableHandleValue) {
		rval.set(ObjectValue(self.obj));
		maybe_wrap_object_value(cx, rval);
	}
}

impl Deref for IonArray {
	type Target = IonRawObject;

	fn deref(&self) -> &Self::Target {
		&self.obj
	}
}

unsafe impl CustomTrace for IonArray {
	fn trace(&self, tracer: *mut JSTracer) {
		self.obj.trace(tracer)
	}
}
