/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use mozjs::jsval::{BooleanValue, DoubleValue, Int32Value, JSVal, NullValue, ObjectValue, UInt32Value, UndefinedValue};
use mozjs::rust::{Handle, MutableHandle};

use crate::{Array, Context, Local, Object};
use crate::conversions::ToValue;

#[derive(Debug)]
pub struct Value<'cx> {
	value: &'cx mut Local<'cx, JSVal>,
}

impl<'cx> Value<'cx> {
	/// Creates a [Value] from a boolean.
	pub fn bool(cx: &'cx Context, b: bool) -> Value<'cx> {
		Value::from(cx.root_value(BooleanValue(b)))
	}

	/// Creates a [Value] from a 32-bit signed integer.
	pub fn i32(cx: &'cx Context, i: i32) -> Value<'cx> {
		Value::from(cx.root_value(Int32Value(i)))
	}

	/// Creates a [Value] from a 32-bit unsigned integer.
	pub fn u32(cx: &'cx Context, u: u32) -> Value<'cx> {
		Value::from(cx.root_value(UInt32Value(u)))
	}

	/// Creates a [Value] from a 64-bit float.
	pub fn f64(cx: &'cx Context, f: f64) -> Value<'cx> {
		Value::from(cx.root_value(DoubleValue(f)))
	}

	/// Creates a [Value] from a string.
	pub fn string(cx: &'cx Context, str: &str) -> Value<'cx> {
		let mut value = Value::from(cx.root_value(UndefinedValue()));
		unsafe { str.to_value(cx, &mut value) };
		value
	}

	/// Creates a [Value] from an [Object].
	pub fn object(cx: &'cx Context, object: &Object) -> Value<'cx> {
		Value::from(cx.root_value(ObjectValue(object.handle().get())))
	}

	/// Creates a [Value] from an [Array].
	pub fn array(cx: &'cx Context, array: &Array) -> Value<'cx> {
		Value::from(cx.root_value(ObjectValue(array.handle().get())))
	}

	/// Creates an `undefined` [Value].
	pub fn undefined(cx: &'cx Context) -> Value<'cx> {
		Value::from(cx.root_value(UndefinedValue()))
	}

	/// Creates an `null` [Value].
	pub fn null(cx: &'cx Context) -> Value<'cx> {
		Value::from(cx.root_value(NullValue()))
	}

	pub fn to_object(&self, cx: &'cx Context) -> Object<'cx> {
		cx.root_object(self.value.to_object()).into()
	}

	pub fn handle<'a>(&'a self) -> Handle<'a, JSVal>
	where
		'cx: 'a,
	{
		self.value.handle()
	}

	pub fn handle_mut<'a>(&'a mut self) -> MutableHandle<'a, JSVal>
	where
		'cx: 'a,
	{
		self.value.handle_mut()
	}
}

impl<'cx> From<&'cx mut Local<'cx, JSVal>> for Value<'cx> {
	fn from(value: &'cx mut Local<'cx, JSVal>) -> Value<'cx> {
		Value { value }
	}
}

impl<'cx> Deref for Value<'cx> {
	type Target = Local<'cx, JSVal>;

	fn deref(&self) -> &Self::Target {
		&*self.value
	}
}

impl<'cx> DerefMut for Value<'cx> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.value
	}
}
