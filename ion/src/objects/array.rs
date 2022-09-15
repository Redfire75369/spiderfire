/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use mozjs::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use mozjs::error::throw_type_error;
use mozjs::jsapi::{
	AssertSameCompartment, GetArrayLength, HandleValueArray, IsArray, JS_DefineElement, JS_DeleteElement1, JS_GetElement, JS_HasElement,
	JS_SetElement, JSObject, JSTracer, NewArrayObject,
};
use mozjs::jsval::{JSVal, ObjectValue, UndefinedValue};
use mozjs::rust::{CustomTrace, HandleValue, maybe_wrap_object_value, MutableHandleValue};

use crate::{Context, Exception};
use crate::flags::PropertyFlags;
use crate::types::values::from_value;

#[derive(Clone, Copy, Debug)]
pub struct Array {
	obj: *mut JSObject,
}

impl Array {
	/// Creates an empty [Array].
	pub fn new(cx: Context) -> Array {
		Array::from_slice(cx, &[])
	}

	/// Creates an [Array] from a [Vec].
	pub fn from_vec(cx: Context, vec: Vec<JSVal>) -> Array {
		Array::from_slice(cx, vec.as_slice())
	}

	/// Creates an [Array] from a slice.
	pub fn from_slice(cx: Context, slice: &[JSVal]) -> Array {
		Array::from_handle(cx, unsafe { HandleValueArray::from_rooted_slice(slice) })
	}

	/// Creates an [Array] from a [HandleValueArray].
	pub fn from_handle(cx: Context, handle: HandleValueArray) -> Array {
		Array::from(cx, unsafe { NewArrayObject(cx, &handle) }).unwrap()
	}

	/// Creates an [Array] from a [*mut JSObject].
	/// Returns [None] if the object is not an array.
	pub fn from(cx: Context, obj: *mut JSObject) -> Option<Array> {
		if Array::is_array_raw(cx, obj) {
			Some(Array { obj })
		} else {
			None
		}
	}

	/// Creates an [Array] from a [JSVal].
	/// Returns [None] if the value is not an object or an array.
	pub fn from_value(cx: Context, val: JSVal) -> Option<Array> {
		if val.is_object() {
			Array::from(cx, val.to_object())
		} else {
			None
		}
	}

	/// Converts an [Array] to a [Vec].
	/// Returns an empty [Vec] if the conversion fails.
	pub fn to_vec(&self, cx: Context) -> Vec<JSVal> {
		rooted!(in(cx) let obj = self.to_value());
		if let ConversionResult::Success(vec) = unsafe { Vec::<JSVal>::from_jsval(cx, obj.handle(), ()).unwrap() } {
			vec
		} else {
			Vec::new()
		}
	}

	/// Converts an [Array] to a [JSVal].
	pub fn to_value(&self) -> JSVal {
		ObjectValue(self.obj)
	}

	/// Returns the length of an [Array].
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self, cx: Context) -> u32 {
		rooted!(in(cx) let robj = self.obj);
		let mut length = 0;
		unsafe {
			GetArrayLength(cx, robj.handle().into(), &mut length);
		}
		length
	}

	/// Checks if the [Array] has a value at the given index.
	pub fn has(&self, cx: Context, index: u32) -> bool {
		rooted!(in(cx) let robj = self.obj);
		let mut found = false;
		if unsafe { JS_HasElement(cx, robj.handle().into(), index, &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Gets the [JSVal] at the given index of the [Array].
	/// Returns [None] if there is no value at the given index.
	pub fn get(&self, cx: Context, index: u32) -> Option<JSVal> {
		if self.has(cx, index) {
			rooted!(in(cx) let robj = self.obj);
			rooted!(in(cx) let mut rval = UndefinedValue());
			unsafe { JS_GetElement(cx, robj.handle().into(), index, rval.handle_mut().into()) };
			Some(rval.get())
		} else {
			None
		}
	}

	/// Gets the value at the given index of the [Array] as a Rust type.
	/// Returns [None] if there is no value at the given index or conversion to the Rust type fails.
	pub fn get_as<T: FromJSValConvertible>(&self, cx: Context, index: u32, config: T::Config) -> Option<T> {
		let opt = self.get(cx, index);
		opt.and_then(|val| from_value(cx, val, config))
	}

	/// Sets the [JSVal] at the given index of the [Array].
	/// Returns `false` if the element cannot be set.
	pub fn set(&mut self, cx: Context, index: u32, value: JSVal) -> bool {
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let rval = value);
		unsafe { JS_SetElement(cx, robj.handle().into(), index, rval.handle().into()) }
	}

	/// Sets the Rust type at the given index of the [Array].
	/// Returns `false` if the element cannot be set.
	pub fn set_as<T: ToJSValConvertible>(&mut self, cx: Context, index: u32, value: T) -> bool {
		rooted!(in(cx) let mut val = UndefinedValue());
		unsafe { value.to_jsval(cx, val.handle_mut()) };
		self.set(cx, index, val.get())
	}

	/// Defines the [JSVal] at the given index of the [Array] with the given attributes.
	/// Returns `false` if the element cannot be defined.
	pub fn define(&mut self, cx: Context, index: u32, value: JSVal, attrs: PropertyFlags) -> bool {
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let rval = value);
		unsafe { JS_DefineElement(cx, robj.handle().into(), index, rval.handle().into(), attrs.bits() as u32) }
	}

	/// Defines the Rust type at the given index of the [Array] with the given attributes.
	/// Returns `false` if the element cannot be defined.
	pub fn define_as<T: ToJSValConvertible>(&mut self, cx: Context, index: u32, value: T, attrs: PropertyFlags) -> bool {
		rooted!(in(cx) let mut val = UndefinedValue());
		unsafe { value.to_jsval(cx, val.handle_mut()) };
		self.define(cx, index, val.get(), attrs)
	}

	/// Deletes the [JSVal] at the given index.
	/// Returns `false` if the element cannot be deleted.
	pub fn delete(&mut self, cx: Context, index: u32) -> bool {
		rooted!(in(cx) let robj = self.obj);
		unsafe { JS_DeleteElement1(cx, robj.handle().into(), index) }
	}

	/// Checks if a [*mut JSObject] is an array.
	pub fn is_array_raw(cx: Context, obj: *mut JSObject) -> bool {
		rooted!(in(cx) let mut robj = obj);
		let mut is_array = false;
		unsafe { IsArray(cx, robj.handle().into(), &mut is_array) && is_array }
	}
}

impl FromJSValConvertible for Array {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: Context, value: HandleValue, _: ()) -> Result<ConversionResult<Array>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "JSVal is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		if let Some(array) = Array::from(cx, value.to_object()) {
			Ok(ConversionResult::Success(array))
		} else {
			Err(())
		}
	}
}

impl ToJSValConvertible for Array {
	#[inline]
	unsafe fn to_jsval(&self, cx: Context, mut rval: MutableHandleValue) {
		rval.set(self.to_value());
		maybe_wrap_object_value(cx, rval);
	}
}

impl Deref for Array {
	type Target = *mut JSObject;

	fn deref(&self) -> &Self::Target {
		&self.obj
	}
}

unsafe impl CustomTrace for Array {
	fn trace(&self, tracer: *mut JSTracer) {
		self.obj.trace(tracer)
	}
}
