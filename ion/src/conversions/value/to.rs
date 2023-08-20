/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ptr::NonNull;
use std::rc::Rc;
use std::string::String as RustString;

use mozjs::jsapi::{JS_GetFunctionObject, JS_WrapValue, JSFunction, JSObject, JSString};
use mozjs::jsapi::PropertyKey as JSPropertyKey;
use mozjs::jsapi::Symbol as JSSymbol;
use mozjs::jsval::{
	BooleanValue, DoubleValue, Int32Value, JSVal, NullValue, ObjectOrNullValue, ObjectValue, StringValue, SymbolValue, UInt32Value, UndefinedValue,
};
use mozjs::rust::{maybe_wrap_object_or_null_value, maybe_wrap_object_value, maybe_wrap_value};
use mozjs_sys::jsapi::JS_IdToValue;

use crate::{Array, Context, Date, Function, Object, Promise, PropertyKey, String, Symbol, Value};

/// Represents types that can be converted to JavaScript [Values](Value).
pub trait ToValue<'cx> {
	/// Converts `self` to a [`Value`](Value) and stores it in `value`.
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value);

	/// Converts `self` to a new [`Value`](value).
	unsafe fn as_value(&self, cx: &'cx Context) -> Value<'cx> {
		let mut value = Value::undefined(cx);
		self.to_value(cx, &mut value);
		value
	}
}

impl ToValue<'_> for () {
	unsafe fn to_value(&self, _: &Context, value: &mut Value) {
		value.handle_mut().set(UndefinedValue());
	}
}

impl ToValue<'_> for bool {
	unsafe fn to_value(&self, _: &Context, value: &mut Value) {
		value.handle_mut().set(BooleanValue(*self));
	}
}

macro_rules! impl_to_value_for_integer {
	($ty:ty, signed) => {
		impl ToValue<'_> for $ty {
			unsafe fn to_value(&self, _: &Context, value: &mut Value) {
				value.handle_mut().set(Int32Value(*self as i32));
			}
		}
	};
	($ty:ty, unsigned) => {
		impl ToValue<'_> for $ty {
			unsafe fn to_value(&self, _: &Context, value: &mut Value) {
				value.handle_mut().set(UInt32Value(*self as u32));
			}
		}
	};
}

impl_to_value_for_integer!(i8, signed);
impl_to_value_for_integer!(i16, signed);
impl_to_value_for_integer!(i32, signed);

impl_to_value_for_integer!(u8, unsigned);
impl_to_value_for_integer!(u16, unsigned);
impl_to_value_for_integer!(u32, unsigned);

macro_rules! impl_to_value_as_double {
	($ty:ty) => {
		impl ToValue<'_> for $ty {
			unsafe fn to_value(&self, _: &Context, value: &mut Value) {
				value.handle_mut().set(DoubleValue(*self as f64));
			}
		}
	};
}

impl_to_value_as_double!(i64);
impl_to_value_as_double!(u64);
impl_to_value_as_double!(f32);
impl_to_value_as_double!(f64);

impl ToValue<'_> for *mut JSString {
	unsafe fn to_value(&self, cx: &Context, value: &mut Value) {
		value.handle_mut().set(StringValue(&**self));
		JS_WrapValue(cx.as_ptr(), value.handle_mut().into());
	}
}

impl<'cx> ToValue<'cx> for String<'cx> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.handle().to_value(cx, value);
	}
}

impl ToValue<'_> for str {
	unsafe fn to_value(&self, cx: &Context, value: &mut Value) {
		let string = String::new(cx, self);
		if let Some(string) = string {
			string.to_value(cx, value);
		} else {
			panic!("Failed to Instantiate String");
		}
	}
}

impl<'cx> ToValue<'cx> for RustString {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		(**self).to_value(cx, value);
	}
}

impl ToValue<'_> for *mut JSObject {
	unsafe fn to_value(&self, cx: &Context, value: &mut Value) {
		value.handle_mut().set(ObjectOrNullValue(*self));
		maybe_wrap_object_or_null_value(cx.as_ptr(), value.handle_mut());
	}
}

impl ToValue<'_> for NonNull<JSObject> {
	unsafe fn to_value(&self, cx: &Context, value: &mut Value) {
		value.handle_mut().set(ObjectValue(self.as_ptr()));
		maybe_wrap_object_value(cx.as_ptr(), value.handle_mut());
	}
}

impl<'cx> ToValue<'cx> for Object<'cx> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.handle().to_value(cx, value);
	}
}

impl<'cx> ToValue<'cx> for Array<'cx> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.handle().to_value(cx, value);
	}
}

impl<'cx> ToValue<'cx> for Date<'cx> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.handle().to_value(cx, value);
	}
}

impl<'cx> ToValue<'cx> for Promise<'cx> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.handle().to_value(cx, value);
	}
}

impl<'cx> ToValue<'cx> for *mut JSFunction {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		JS_GetFunctionObject(*self).to_value(cx, value);
	}
}

impl<'cx> ToValue<'cx> for Function<'cx> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.handle().to_value(cx, value);
	}
}

impl ToValue<'_> for *mut JSSymbol {
	unsafe fn to_value(&self, _: &Context, value: &mut Value) {
		value.handle_mut().set(SymbolValue(&**self));
	}
}

impl<'cx> ToValue<'cx> for Symbol<'cx> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.handle().to_value(cx, value);
	}
}

impl ToValue<'_> for JSVal {
	unsafe fn to_value(&self, cx: &Context, value: &mut Value) {
		value.handle_mut().set(*self);
		maybe_wrap_value(cx.as_ptr(), value.handle_mut());
	}
}

impl<'cx> ToValue<'cx> for Value<'cx> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.handle().to_value(cx, value);
	}
}

impl<'cx> ToValue<'cx> for JSPropertyKey {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		JS_IdToValue(cx.as_ptr(), *self, value.handle_mut().into());
	}
}

impl<'cx> ToValue<'cx> for PropertyKey<'cx> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.handle().to_value(cx, value);
	}
}

impl<'cx, T: ToValue<'cx>> ToValue<'cx> for &'_ T {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		(*self).to_value(cx, value);
	}
}

impl<'cx, T: ToValue<'cx>> ToValue<'cx> for Box<T> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		(**self).to_value(cx, value);
	}
}

impl<'cx, T: ToValue<'cx>> ToValue<'cx> for Rc<T> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		(**self).to_value(cx, value);
	}
}

impl<'cx, T: ToValue<'cx>> ToValue<'cx> for Option<T> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		match self {
			Some(t) => t.to_value(cx, value),
			None => value.handle_mut().set(NullValue()),
		};
	}
}

impl<'cx, T: ToValue<'cx>> ToValue<'cx> for [T] {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		let mut array = Array::new_with_length(cx, self.len());

		for (i, t) in self.iter().enumerate() {
			assert!(array.set_as(cx, i as u32, t));
		}

		array.to_value(cx, value);
	}
}

impl<'cx, T: ToValue<'cx>> ToValue<'cx> for Vec<T> {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		(**self).to_value(cx, value);
	}
}
