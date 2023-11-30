/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;
use std::ptr::NonNull;
use std::rc::Rc;

use mozjs::jsapi::{JS_GetFunctionObject, JS_IdToValue, JS_NewStringCopyN, JS_WrapValue, JSFunction, JSObject, JSString};
use mozjs::jsapi::PropertyKey as JSPropertyKey;
use mozjs::jsapi::Symbol as JSSymbol;
use mozjs::jsval::{
	BooleanValue, DoubleValue, Int32Value, JSVal, NullValue, ObjectOrNullValue, StringValue, SymbolValue, UInt32Value,
	UndefinedValue,
};
use mozjs::rust::MutableHandle;

use crate::{Array, Context, Date, Error, ErrorKind, Function, Object, Promise, PropertyKey, Result, Symbol, Value};
use crate::objects::RegExp;
use crate::string::byte::{BytePredicate, ByteStr, ByteString};

/// Represents types that can be converted to JavaScript [Values](Value).
pub trait ToValue {
	/// Converts `self` to a new [`Value`](Value).
	fn to_value(&self, cx: &Context) -> Result<Value>;
}

impl ToValue for () {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		Ok(Value::from(cx.root(UndefinedValue())))
	}
}

impl ToValue for bool {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		Ok(Value::from(cx.root(BooleanValue(*self))))
	}
}

macro_rules! impl_to_value_for_integer {
	($ty:ty, signed) => {
		impl ToValue for $ty {
			fn to_value(&self, cx: &Context) -> Result<Value> {
				Ok(Value::from(cx.root(Int32Value(*self as i32))))
			}
		}
	};
	($ty:ty, unsigned) => {
		impl ToValue for $ty {
			fn to_value(&self, cx: &Context) -> Result<Value> {
				Ok(Value::from(cx.root(UInt32Value(*self as u32))))
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
		impl ToValue for $ty {
			fn to_value(&self, cx: &Context) -> Result<Value> {
				Ok(Value::from(cx.root(DoubleValue(*self as f64))))
			}
		}
	};
}

impl_to_value_as_double!(i64);
impl_to_value_as_double!(u64);
impl_to_value_as_double!(f32);
impl_to_value_as_double!(f64);

fn wrap_value(cx: &Context, value: MutableHandle<JSVal>) -> Result<Value> {
	if unsafe { JS_WrapValue(cx.as_ptr(), value.into()) } {
		Ok(Value::from(cx.root(value.get())))
	} else {
		Err(Error::new("Failure while wrapping value", ErrorKind::Type))
	}
}

impl ToValue for *mut JSString {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		rooted!(in(cx.as_ptr()) let mut value = StringValue(unsafe { &**self }));
		wrap_value(cx, value.handle_mut())
	}
}

impl ToValue for crate::String {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl ToValue for str {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		let string = crate::String::copy_from_str(cx, self);
		if let Some(string) = string {
			string.to_value(cx)
		} else {
			Err(Error::new("Failed to copy data from string", ErrorKind::Type))
		}
	}
}

impl ToValue for String {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		(**self).to_value(cx)
	}
}

impl<T: ToOwned + ToValue> ToValue for Cow<'_, T> {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.as_ref().to_value(cx)
	}
}

impl<T: BytePredicate> ToValue for ByteStr<T> {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		unsafe { JS_NewStringCopyN(cx.as_ptr(), self.as_ptr() as *const _, self.len()).to_value(cx) }
	}
}

impl<T: BytePredicate> ToValue for ByteString<T> {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		(**self).to_value(cx)
	}
}

impl ToValue for *mut JSObject {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		rooted!(in(cx.as_ptr()) let mut value = ObjectOrNullValue(*self));
		wrap_value(cx, value.handle_mut())
	}
}

impl ToValue for NonNull<JSObject> {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.as_ptr().to_value(cx)
	}
}

impl ToValue for Object {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl ToValue for Array {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl ToValue for Date {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl ToValue for Promise {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl ToValue for RegExp {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl ToValue for *mut JSFunction {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		unsafe { JS_GetFunctionObject(*self) }.to_value(cx)
	}
}

impl ToValue for Function {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl ToValue for *mut JSSymbol {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		Ok(Value::from(cx.root(SymbolValue(unsafe { &**self }))))
	}
}

impl ToValue for Symbol {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl ToValue for JSVal {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		rooted!(in(cx.as_ptr()) let mut value = *self);
		wrap_value(cx, value.handle_mut())
	}
}

impl ToValue for Value {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl ToValue for JSPropertyKey {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		rooted!(in(cx.as_ptr()) let mut value = UndefinedValue());
		unsafe {
			JS_IdToValue(cx.as_ptr(), *self, value.handle_mut().into());
		}
		wrap_value(cx, value.handle_mut())
	}
}

impl ToValue for PropertyKey {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		self.handle().to_value(cx)
	}
}

impl<T: ToValue + ?Sized> ToValue for &T {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		(*self).to_value(cx)
	}
}

impl<T: ToValue + ?Sized> ToValue for Box<T> {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		(**self).to_value(cx)
	}
}

impl<T: ToValue + ?Sized> ToValue for Rc<T> {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		(**self).to_value(cx)
	}
}

impl<T: ToValue> ToValue for Option<T> {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		match self {
			Some(t) => t.to_value(cx),
			None => Ok(Value::from(cx.root(NullValue()))),
		}
	}
}

impl<T: ToValue> ToValue for [T] {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		let mut array = Array::new_with_length(cx, self.len());

		for (i, t) in self.iter().enumerate() {
			assert!(array.set_as(cx, i as u32, t).is_ok());
		}

		array.to_value(cx)
	}
}

impl<T: ToValue> ToValue for Vec<T> {
	fn to_value(&self, cx: &Context) -> Result<Value> {
		(**self).to_value(cx)
	}
}
