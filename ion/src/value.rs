/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use mozjs::jsval::{BooleanValue, DoubleValue, Int32Value, JSVal, NullValue, ObjectValue, UInt32Value, UndefinedValue};

use crate::{Array, Context, Local, Object};
use crate::conversions::ToValue;

/// Represents a JavaScript Value in the runtime.
/// It can represent either a primitive or an object.
#[derive(Debug)]
pub struct Value<'v> {
	val: Local<'v, JSVal>,
}

impl<'v> Value<'v> {
	/// Creates a [Value] from a boolean.
	pub fn bool<'cx>(cx: &'cx Context, b: bool) -> Value<'cx> {
		Value::from(cx.root_value(BooleanValue(b)))
	}

	/// Creates a [Value] from a 32-bit signed integer.
	pub fn i32<'cx>(cx: &'cx Context, i: i32) -> Value<'cx> {
		Value::from(cx.root_value(Int32Value(i)))
	}

	/// Creates a [Value] from a 32-bit unsigned integer.
	pub fn u32<'cx>(cx: &'cx Context, u: u32) -> Value<'cx> {
		Value::from(cx.root_value(UInt32Value(u)))
	}

	/// Creates a [Value] from a 64-bit float.
	pub fn f64<'cx>(cx: &'cx Context, f: f64) -> Value<'cx> {
		Value::from(cx.root_value(DoubleValue(f)))
	}

	/// Creates a [Value] from a string.
	pub fn string<'cx>(cx: &'cx Context, str: &str) -> Value<'cx> {
		unsafe { str.as_value(cx) }
	}

	/// Creates a [Value] from an [Object].
	pub fn object<'cx>(cx: &'cx Context, object: &Object) -> Value<'cx> {
		Value::from(cx.root_value(ObjectValue(object.handle().get())))
	}

	/// Creates a [Value] from an [Array].
	pub fn array<'cx>(cx: &'cx Context, array: &Array) -> Value<'cx> {
		Value::from(cx.root_value(ObjectValue(array.handle().get())))
	}

	/// Creates an `undefined` [Value].
	pub fn undefined<'cx>(cx: &'cx Context) -> Value<'cx> {
		Value::from(cx.root_value(UndefinedValue()))
	}

	/// Creates a `null` [Value].
	pub fn null<'cx>(cx: &'cx Context) -> Value<'cx> {
		Value::from(cx.root_value(NullValue()))
	}

	/// Converts a [Value] to an [Object].
	///
	/// ### Panics
	/// This panics if the [Value] is not an object.
	pub fn to_object<'cx: 'v>(&self, cx: &'cx Context) -> Object<'cx> {
		cx.root_object(self.handle().to_object()).into()
	}
}

impl<'v> From<Local<'v, JSVal>> for Value<'v> {
	fn from(val: Local<'v, JSVal>) -> Value<'v> {
		Value { val }
	}
}

impl<'v> Deref for Value<'v> {
	type Target = Local<'v, JSVal>;

	fn deref(&self) -> &Self::Target {
		&self.val
	}
}

impl<'v> DerefMut for Value<'v> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.val
	}
}
