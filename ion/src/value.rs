/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use mozjs::conversions::ToJSValConvertible;
use mozjs::jsval::{BooleanValue, DoubleValue, Int32Value, JSVal, NullValue, UInt32Value, UndefinedValue};
use mozjs::rust::MutableHandleValue;

use crate::Context;

pub struct Value {
	val: JSVal,
}

impl Value {
	/// Creates a [Value] from a [JSVal].
	pub fn from_raw(val: JSVal) -> Value {
		Value { val }
	}

	/// Creates a [Value] from a boolean.
	pub fn bool(b: bool) -> Value {
		Value::from_raw(BooleanValue(b))
	}

	/// Creates a [Value] from a 32-bit signed integer.
	pub fn i32(i: i32) -> Value {
		Value::from_raw(Int32Value(i))
	}

	/// Creates a [Value] from a 32-bit unsigned integer.
	pub fn u32(u: u32) -> Value {
		Value::from_raw(UInt32Value(u))
	}

	/// Creates a [Value] from a double.
	pub fn f64(f: f64) -> Value {
		Value::from_raw(DoubleValue(f))
	}

	/// Creates a [Value] from a [String].
	pub fn string(cx: Context, str: String) -> Value {
		rooted!(in(cx) let mut val = *Value::undefined());
		unsafe { str.to_jsval(cx, val.handle_mut()) };
		Value::from_raw(val.get())
	}

	/// Creates an `undefined` [Value].
	pub fn undefined() -> Value {
		Value::from_raw(UndefinedValue())
	}

	/// Creates an `null` [Value].
	pub fn null() -> Value {
		Value::from_raw(NullValue())
	}
}

impl ToJSValConvertible for Value {
	#[inline]
	unsafe fn to_jsval(&self, _: Context, mut rval: MutableHandleValue) {
		rval.set(self.val);
	}
}

impl Deref for Value {
	type Target = JSVal;

	fn deref(&self) -> &Self::Target {
		&self.val
	}
}

impl DerefMut for Value {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.val
	}
}
