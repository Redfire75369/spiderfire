/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use mozjs::jsapi::{Heap, JS_ValueToSource, SameValue};
use mozjs::jsval::{
	BigIntValue, BooleanValue, DoubleValue, Int32Value, JSVal, NullValue, ObjectValue, SymbolValue, UInt32Value,
	UndefinedValue,
};

use crate::{Array, Context, Object, Root, Symbol};
use crate::bigint::BigInt;
use crate::conversions::ToValue;

/// Represents a JavaScript Value in the runtime.
/// It can represent either a primitive or an object.
#[derive(Debug)]
pub struct Value {
	val: Root<Box<Heap<JSVal>>>,
}

impl Value {
	/// Creates a [Value] from a boolean.
	pub fn bool(cx: &Context, b: bool) -> Value {
		Value::from(cx.root(BooleanValue(b)))
	}

	/// Creates a [Value] from a 32-bit signed integer.
	pub fn i32(cx: &Context, i: i32) -> Value {
		Value::from(cx.root(Int32Value(i)))
	}

	/// Creates a [Value] from a 32-bit unsigned integer.
	pub fn u32(cx: &Context, u: u32) -> Value {
		Value::from(cx.root(UInt32Value(u)))
	}

	/// Creates a [Value] from a 64-bit float.
	pub fn f64(cx: &Context, f: f64) -> Value {
		Value::from(cx.root(DoubleValue(f)))
	}

	/// Creates a [Value] from a string.
	pub fn string(cx: &Context, str: &str) -> Value {
		str.to_value(cx).unwrap()
	}

	/// Creates a [Value] from a [BigInt].
	pub fn bigint(cx: &Context, bi: &BigInt) -> Value {
		Value::from(cx.root(BigIntValue(unsafe { &*bi.get() })))
	}

	/// Creates a [Value] from a [Symbol].
	pub fn symbol(cx: &Context, sym: &Symbol) -> Value {
		Value::from(cx.root(SymbolValue(unsafe { &*sym.get() })))
	}

	/// Creates a [Value] from an [Object].
	pub fn object(cx: &Context, object: &Object) -> Value {
		Value::from(cx.root(ObjectValue(object.handle().get())))
	}

	/// Creates a [Value] from an [Array].
	pub fn array(cx: &Context, array: &Array) -> Value {
		Value::from(cx.root(ObjectValue(array.handle().get())))
	}

	/// Creates an `undefined` [Value].
	pub fn undefined(cx: &Context) -> Value {
		Value::from(cx.root(UndefinedValue()))
	}

	/// Creates a `null` [Value].
	pub fn null(cx: &Context) -> Value {
		Value::from(cx.root(NullValue()))
	}

	/// Converts a [Value] to an [Object].
	///
	/// ### Panics
	/// This panics if the [Value] is not an object.
	pub fn to_object(&self, cx: &Context) -> Object {
		cx.root(self.handle().to_object()).into()
	}

	/// Compares two values for equality using the [SameValue algorithm](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-samevalue).
	/// This is identical to strict equality (===), except that NaN's are equal and 0 !== -0.
	pub fn is_same(&self, cx: &Context, other: &Value) -> bool {
		let mut same = false;
		unsafe { SameValue(cx.as_ptr(), self.handle().into(), other.handle().into(), &mut same) && same }
	}

	pub fn to_source(&self, cx: &Context) -> crate::String {
		crate::String::from(cx.root(unsafe { JS_ValueToSource(cx.as_ptr(), self.handle().into()) }))
	}
}

impl From<Root<Box<Heap<JSVal>>>> for Value {
	fn from(val: Root<Box<Heap<JSVal>>>) -> Value {
		Value { val }
	}
}

impl Deref for Value {
	type Target = Root<Box<Heap<JSVal>>>;

	fn deref(&self) -> &Self::Target {
		&self.val
	}
}

impl DerefMut for Value {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.val
	}
}
