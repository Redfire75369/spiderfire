/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use mozjs::jsapi::{
	GetArrayLength, HandleValueArray, IsArray, JS_DefineElement, JS_DeleteElement1, JS_GetElement, JS_HasElement, JS_SetElement, JSObject,
	NewArrayObject, NewArrayObject1,
};
use mozjs::jsval::{JSVal, ObjectValue, UndefinedValue};
use mozjs::rust::{Handle, MutableHandle};

use crate::{Context, Exception, Local, Value};
use crate::conversions::{FromValue, ToValue};
use crate::flags::PropertyFlags;

#[derive(Debug)]
pub struct Array<'a> {
	array: Local<'a, *mut JSObject>,
}

impl<'a> Array<'a> {
	/// Creates an empty [Array].
	pub fn new<'cx>(cx: &'cx Context) -> Array<'cx> {
		Array::new_with_length(cx, 0)
	}

	pub fn new_with_length<'cx>(cx: &'cx Context, length: usize) -> Array<'cx> {
		Array {
			array: cx.root_object(unsafe { NewArrayObject1(**cx, length) }),
		}
	}

	/// Creates an [Array] from a [Vec] of JSVal.
	pub fn from_vec<'cx>(cx: &'cx Context, vec: Vec<JSVal>) -> Array<'cx> {
		Array::from_slice(cx, vec.as_slice())
	}

	/// Creates an [Array] from a slice.
	pub fn from_slice<'cx>(cx: &'cx Context, slice: &[JSVal]) -> Array<'cx> {
		Array::from_handle(cx, unsafe { HandleValueArray::from_rooted_slice(slice) })
	}

	/// Creates an [Array] from a [HandleValueArray].
	pub fn from_handle<'cx>(cx: &'cx Context, handle: HandleValueArray) -> Array<'cx> {
		Array {
			array: cx.root_object(unsafe { NewArrayObject(**cx, &handle) }),
		}
	}

	/// Creates an [Array] from an object.
	/// Returns [None] if the object is not an array.
	pub fn from(cx: &Context, object: Local<'a, *mut JSObject>) -> Option<Array<'a>> {
		if Array::is_array(cx, &object) {
			Some(Array { array: object })
		} else {
			None
		}
	}

	/// Creates an [Array] from an object.
	///
	/// ### Safety
	/// Object must be an array.
	pub unsafe fn from_unchecked(object: Local<'a, *mut JSObject>) -> Array<'a> {
		Array { array: object }
	}

	/// Converts an [Array] to a [Vec].
	/// Returns an empty [Vec] if the conversion fails.
	pub fn to_vec<'cx>(&self, cx: &'cx Context) -> Vec<Value<'cx>> {
		let value = cx.root_value(ObjectValue(*self.array)).into();
		if let Ok(vec) = unsafe { Vec::from_value(cx, &value, true, ()) } {
			vec
		} else {
			Vec::new()
		}
	}

	/// Returns the length of an [Array].
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self, cx: &Context) -> u32 {
		let mut length = 0;
		unsafe {
			GetArrayLength(**cx, self.handle().into(), &mut length);
		}
		length
	}

	/// Checks if the [Array] has a value at the given index.
	pub fn has(&self, cx: &Context, index: u32) -> bool {
		let mut found = false;
		if unsafe { JS_HasElement(**cx, self.handle().into(), index, &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Gets the [JSVal] at the given index of the [Array].
	/// Returns [None] if there is no value at the given index.
	pub fn get<'cx>(&self, cx: &'cx Context, index: u32) -> Option<Value<'cx>> {
		if self.has(cx, index) {
			let mut rval = Value::from(cx.root_value(UndefinedValue()));
			unsafe { JS_GetElement(**cx, self.handle().into(), index, rval.handle_mut().into()) };
			Some(rval)
		} else {
			None
		}
	}

	/// Gets the value at the given index of the [Array] as a Rust type.
	/// Returns [None] if there is no value at the given index or conversion to the Rust type fails.
	pub fn get_as<'cx, T: FromValue<'cx>>(&self, cx: &'cx Context, index: u32, strict: bool, config: T::Config) -> Option<T> {
		self.get(cx, index)
			.and_then(|val| unsafe { T::from_value(cx, &val, strict, config).ok() })
	}

	/// Sets the [JSVal] at the given index of the [Array].
	/// Returns `false` if the element cannot be set.
	pub fn set(&mut self, cx: &Context, index: u32, value: &Value) -> bool {
		unsafe { JS_SetElement(**cx, self.handle().into(), index, value.handle().into()) }
	}

	/// Sets the Rust type at the given index of the [Array].
	/// Returns `false` if the element cannot be set.
	pub fn set_as<'cx, T: ToValue<'cx>>(&mut self, cx: &'cx Context, index: u32, value: T) -> bool {
		let mut val = Value::undefined(cx);
		unsafe { value.to_value(cx, &mut val) };
		self.set(cx, index, &val)
	}

	/// Defines the [JSVal] at the given index of the [Array] with the given attributes.
	/// Returns `false` if the element cannot be defined.
	pub fn define(&mut self, cx: &Context, index: u32, value: &Value, attrs: PropertyFlags) -> bool {
		unsafe { JS_DefineElement(**cx, self.handle().into(), index, value.handle().into(), attrs.bits() as u32) }
	}

	/// Defines the Rust type at the given index of the [Array] with the given attributes.
	/// Returns `false` if the element cannot be defined.
	pub fn define_as<'cx, T: ToValue<'cx>>(&mut self, cx: &'cx Context, index: u32, value: T, attrs: PropertyFlags) -> bool {
		let mut val = Value::undefined(cx);
		unsafe { value.to_value(cx, &mut val) };
		self.define(cx, index, &val, attrs)
	}

	/// Deletes the [JSVal] at the given index.
	/// Returns `false` if the element cannot be deleted.
	pub fn delete(&mut self, cx: &Context, index: u32) -> bool {
		unsafe { JS_DeleteElement1(**cx, self.handle().into(), index) }
	}

	pub fn handle<'s>(&'s self) -> Handle<'s, *mut JSObject>
	where
		'a: 's,
	{
		self.array.handle()
	}

	pub fn handle_mut<'s>(&'s mut self) -> MutableHandle<'s, *mut JSObject>
	where
		'a: 's,
	{
		self.array.handle_mut()
	}

	/// Checks if a [*mut JSObject] is an array.
	pub fn is_array_raw(cx: &Context, object: *mut JSObject) -> bool {
		rooted!(in(**cx) let object = object);
		let mut is_array = false;
		unsafe { IsArray(**cx, object.handle().into(), &mut is_array) && is_array }
	}

	/// Checks if a [*mut JSObject] is an array.
	pub fn is_array(cx: &Context, object: &Local<*mut JSObject>) -> bool {
		let mut is_array = false;
		unsafe { IsArray(**cx, object.handle().into(), &mut is_array) && is_array }
	}
}

impl<'a> Deref for Array<'a> {
	type Target = Local<'a, *mut JSObject>;

	fn deref(&self) -> &Self::Target {
		&self.array
	}
}
