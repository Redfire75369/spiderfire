/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use mozjs::jsapi::{GetArrayLength, HandleValueArray, IsArray, JSObject, NewArrayObject, NewArrayObject1};
use mozjs::jsval::{JSVal, ObjectValue};

use crate::{Context, Local, Object, Value};
use crate::conversions::{FromValue, ToValue};
use crate::flags::PropertyFlags;

/// Represents an [Array] in the JavaScript Runtime.
/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array) for more details.
#[derive(Debug)]
pub struct Array<'a> {
	arr: Object<'a>,
}

impl<'a> Array<'a> {
	/// Creates an empty [Array].
	pub fn new<'cx>(cx: &'cx Context) -> Array<'cx> {
		Array::new_with_length(cx, 0)
	}

	/// Creates an empty [Array] with the given length.
	pub fn new_with_length<'cx>(cx: &'cx Context, length: usize) -> Array<'cx> {
		Array {
			arr: cx.root_object(unsafe { NewArrayObject1(cx.as_ptr(), length) }).into(),
		}
	}

	/// Creates an [Array] from a slice of values.
	pub fn from_slice<'cx>(cx: &'cx Context, slice: &[JSVal]) -> Array<'cx> {
		Array::from_handle(cx, unsafe { HandleValueArray::from_rooted_slice(slice) })
	}

	/// Creates an [Array] from a [HandleValueArray].
	pub fn from_handle<'cx>(cx: &'cx Context, handle: HandleValueArray) -> Array<'cx> {
		Array {
			arr: cx.root_object(unsafe { NewArrayObject(cx.as_ptr(), &handle) }).into(),
		}
	}

	/// Creates an [Array] from an object.
	///
	/// Returns [None] if the object is not an array.
	pub fn from(cx: &Context, object: Local<'a, *mut JSObject>) -> Option<Array<'a>> {
		if Array::is_array(cx, &object) {
			Some(Array { arr: object.into() })
		} else {
			None
		}
	}

	/// Creates an [Array] from an object.
	///
	/// ### Safety
	/// Object must be an array.
	pub unsafe fn from_unchecked(object: Local<'a, *mut JSObject>) -> Array<'a> {
		Array { arr: object.into() }
	}

	/// Converts an [Array] to a [Vec].
	/// Returns an empty [Vec] if the conversion fails.
	pub fn to_vec<'cx>(&self, cx: &'cx Context) -> Vec<Value<'cx>> {
		let value = cx.root_value(ObjectValue(self.arr.handle().get())).into();
		if let Ok(vec) = unsafe { Vec::from_value(cx, &value, true, ()) } {
			vec
		} else {
			Vec::new()
		}
	}

	/// Converts an [Array] to an [Object].
	pub fn to_object<'cx>(&self, cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(self.arr.handle().get()))
	}

	/// Returns the length of the [Array].
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self, cx: &Context) -> u32 {
		let mut length = 0;
		unsafe {
			GetArrayLength(cx.as_ptr(), self.handle().into(), &mut length);
		}
		length
	}

	/// Checks if the [Array] has a value at the given index.
	pub fn has(&self, cx: &Context, index: u32) -> bool {
		self.arr.has(cx, index)
	}

	/// Gets the [Value] at the given index of the [Array].
	/// Returns [None] if there is no value at the given index.
	pub fn get<'cx>(&self, cx: &'cx Context, index: u32) -> Option<Value<'cx>> {
		self.arr.get(cx, index)
	}

	/// Gets the value at the given index of the [Array] as a Rust type.
	/// Returns [None] if there is no value at the given index or conversion to the Rust type fails.
	pub fn get_as<'cx, T: FromValue<'cx>>(&self, cx: &'cx Context, index: u32, strict: bool, config: T::Config) -> Option<T> {
		self.arr.get_as(cx, index, strict, config)
	}

	/// Sets the [Value] at the given index of the [Array].
	/// Returns `false` if the element cannot be set.
	pub fn set(&mut self, cx: &Context, index: u32, value: &Value) -> bool {
		self.arr.set(cx, index, value)
	}

	/// Sets the Rust type at the given index of the [Array].
	/// Returns `false` if the element cannot be set.
	pub fn set_as<'cx, T: ToValue<'cx> + ?Sized>(&mut self, cx: &'cx Context, index: u32, value: &T) -> bool {
		self.arr.set_as(cx, index, value)
	}

	/// Defines the [Value] at the given index of the [Array] with the given attributes.
	/// Returns `false` if the element cannot be defined.
	pub fn define(&mut self, cx: &Context, index: u32, value: &Value, attrs: PropertyFlags) -> bool {
		self.arr.define(cx, index, value, attrs)
	}

	/// Defines the Rust type at the given index of the [Array] with the given attributes.
	/// Returns `false` if the element cannot be defined.
	pub fn define_as<'cx, T: ToValue<'cx> + ?Sized>(&mut self, cx: &'cx Context, index: u32, value: &T, attrs: PropertyFlags) -> bool {
		self.arr.define_as(cx, index, value, attrs)
	}

	/// Deletes the [JSVal] at the given index.
	/// Returns `false` if the element cannot be deleted.
	pub fn delete<'cx: 'a>(&mut self, cx: &'cx Context, index: u32) -> bool {
		self.arr.delete(cx, index)
	}

	/// Checks if a [*mut JSObject] is an array.
	pub fn is_array_raw(cx: &Context, object: *mut JSObject) -> bool {
		rooted!(in(cx.as_ptr()) let object = object);
		let mut is_array = false;
		unsafe { IsArray(cx.as_ptr(), object.handle().into(), &mut is_array) && is_array }
	}

	/// Checks if a [*mut JSObject] is an array.
	pub fn is_array(cx: &Context, object: &Local<*mut JSObject>) -> bool {
		let mut is_array = false;
		unsafe { IsArray(cx.as_ptr(), object.handle().into(), &mut is_array) && is_array }
	}
}

impl<'a> Deref for Array<'a> {
	type Target = Local<'a, *mut JSObject>;

	fn deref(&self) -> &Self::Target {
		&self.arr
	}
}

impl<'a> DerefMut for Array<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.arr
	}
}
