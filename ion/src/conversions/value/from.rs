/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::string::String as RustString;

use mozjs::conversions::{ConversionResult, FromJSValConvertible};
pub use mozjs::conversions::ConversionBehavior;
use mozjs::jsapi::{
	AssertSameCompartment, AssertSameCompartment1, ForOfIterator, ForOfIterator_NonIterableBehavior, JSFunction, JSObject, JSString, RootedObject,
	RootedValue,
};
use mozjs::jsapi::Symbol as JSSymbol;
use mozjs::jsval::JSVal;
use mozjs::rust::{ToBoolean, ToNumber, ToString};
use mozjs::typedarray::{JSObjectStorage, TypedArray, TypedArrayElement};

use crate::{Array, Context, Date, Error, ErrorKind, Exception, Function, Object, Promise, Result, String, Symbol, Value};

/// Represents types that can be converted to from [JavaScript Values](Value).
pub trait FromValue<'cx>: Sized {
	type Config;

	/// Converts `value` to the desired type.
	/// `strict` and `config` determine the strictness of the conversion and specify additional conversion constraints respectively.
	/// Returns [Err] with the [error](Error) if conversion fails.
	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, config: Self::Config) -> Result<Self>
	where
		'cx: 'v;
}

impl<'cx> FromValue<'cx> for bool {
	type Config = ();

	unsafe fn from_value<'v>(_: &'cx Context, value: &Value<'v>, strict: bool, _: ()) -> Result<bool>
	where
		'cx: 'v,
	{
		let value = value.handle();
		if value.is_boolean() {
			return Ok(value.to_boolean());
		}

		if strict {
			Err(Error::new("Expected Boolean in Strict Conversion", ErrorKind::Type))
		} else {
			Ok(ToBoolean(value))
		}
	}
}

macro_rules! impl_from_value_for_integer {
	($ty:ty) => {
		impl<'cx> FromValue<'cx> for $ty {
			type Config = ConversionBehavior;

			unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, config: ConversionBehavior) -> Result<$ty>
			where
				'cx: 'v,
			{
				let value = value.handle();
				if strict && !value.is_number() {
					return Err(Error::new("Expected Number in Strict Conversion", ErrorKind::Type));
				}

				match <$ty>::from_jsval(cx.as_ptr(), value, config) {
					Ok(ConversionResult::Success(number)) => Ok(number),
					Err(_) => Err(Exception::new(cx).unwrap().to_error()),
					_ => unreachable!(),
				}
			}
		}
	};
}

impl_from_value_for_integer!(u8);
impl_from_value_for_integer!(u16);
impl_from_value_for_integer!(u32);
impl_from_value_for_integer!(u64);

impl_from_value_for_integer!(i8);
impl_from_value_for_integer!(i16);
impl_from_value_for_integer!(i32);
impl_from_value_for_integer!(i64);

impl<'cx> FromValue<'cx> for f32 {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, _: ()) -> Result<f32>
	where
		'cx: 'v,
	{
		f64::from_value(cx, value, strict, ()).map(|float| float as f32)
	}
}

impl<'cx> FromValue<'cx> for f64 {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, _: ()) -> Result<f64>
	where
		'cx: 'v,
	{
		let value = value.handle();
		if strict && !value.is_number() {
			return Err(Error::new("Expected Number in Strict Conversion", ErrorKind::Type));
		}

		ToNumber(cx.as_ptr(), value).map_err(|_| Error::new("Unable to Convert Value to Number", ErrorKind::Type))
	}
}

impl<'cx> FromValue<'cx> for *mut JSString {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, _: ()) -> Result<*mut JSString>
	where
		'cx: 'v,
	{
		let value = value.handle();
		if strict && !value.is_string() {
			return Err(Error::new("Expected String in Strict Conversion", ErrorKind::Type));
		}
		Ok(ToString(cx.as_ptr(), value))
	}
}

impl<'cx> FromValue<'cx> for String<'cx> {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, config: ()) -> Result<String<'cx>>
	where
		'cx: 'v,
	{
		<*mut JSString>::from_value(cx, value, strict, config).map(|str| String::from(cx.root_string(str)))
	}
}

impl<'cx> FromValue<'cx> for RustString {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, config: ()) -> Result<RustString>
	where
		'cx: 'v,
	{
		String::from_value(cx, value, strict, config).map(|s| s.to_owned(cx))
	}
}

impl<'cx> FromValue<'cx> for *mut JSObject {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<*mut JSObject>
	where
		'cx: 'v,
	{
		let value = value.handle();
		if !value.is_object() {
			return Err(Error::new("Expected Object", ErrorKind::Type));
		}
		let object = value.to_object();
		AssertSameCompartment(cx.as_ptr(), object);

		Ok(object)
	}
}

impl<'cx> FromValue<'cx> for Object<'cx> {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<Object<'cx>>
	where
		'cx: 'v,
	{
		if !value.handle().is_object() {
			return Err(Error::new("Expected Object", ErrorKind::Type));
		}
		let object = value.to_object(cx);
		AssertSameCompartment(cx.as_ptr(), object.handle().get());

		Ok(object)
	}
}

impl<'cx> FromValue<'cx> for Array<'cx> {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<Array<'cx>>
	where
		'cx: 'v,
	{
		if !value.handle().is_object() {
			return Err(Error::new("Expected Array", ErrorKind::Type));
		}

		let object = value.to_object(cx).into_local();
		if let Some(array) = Array::from(cx, object) {
			AssertSameCompartment(cx.as_ptr(), array.handle().get());
			Ok(array)
		} else {
			Err(Error::new("Expected Array", ErrorKind::Type))
		}
	}
}

impl<'cx> FromValue<'cx> for Date<'cx> {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<Date<'cx>>
	where
		'cx: 'v,
	{
		if !value.handle().is_object() {
			return Err(Error::new("Expected Date", ErrorKind::Type));
		}

		let object = value.to_object(cx).into_local();
		if let Some(date) = Date::from(cx, object) {
			AssertSameCompartment(cx.as_ptr(), date.get());
			Ok(date)
		} else {
			Err(Error::new("Expected Date", ErrorKind::Type))
		}
	}
}

impl<'cx> FromValue<'cx> for Promise<'cx> {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<Promise<'cx>>
	where
		'cx: 'v,
	{
		if !value.handle().is_object() {
			return Err(Error::new("Expected Promise", ErrorKind::Type));
		}

		let object = value.to_object(cx).into_local();
		if let Some(promise) = Promise::from(object) {
			AssertSameCompartment(cx.as_ptr(), promise.get());
			Ok(promise)
		} else {
			Err(Error::new("Expected Promise", ErrorKind::Type))
		}
	}
}

impl<'cx> FromValue<'cx> for *mut JSFunction {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, config: ()) -> Result<*mut JSFunction>
	where
		'cx: 'v,
	{
		Function::from_value(cx, value, strict, config).map(|f| f.get())
	}
}

impl<'cx> FromValue<'cx> for Function<'cx> {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<Function<'cx>>
	where
		'cx: 'v,
	{
		if !value.handle().is_object() {
			return Err(Error::new("Expected Function", ErrorKind::Type));
		}

		let function_obj = value.to_object(cx);
		if let Some(function) = Function::from_object(cx, &function_obj) {
			AssertSameCompartment(cx.as_ptr(), function_obj.handle().get());
			Ok(function)
		} else {
			Err(Error::new("Expected Function", ErrorKind::Type))
		}
	}
}

impl<'cx> FromValue<'cx> for *mut JSSymbol {
	type Config = ();

	unsafe fn from_value<'v>(_: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<*mut JSSymbol>
	where
		'cx: 'v,
	{
		let value = value.handle();
		if value.is_symbol() {
			Ok(value.to_symbol())
		} else {
			Err(Error::new("Expected Symbol", ErrorKind::Type))
		}
	}
}

impl<'cx> FromValue<'cx> for Symbol<'cx> {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, config: Self::Config) -> Result<Symbol<'cx>>
	where
		'cx: 'v,
	{
		<*mut JSSymbol>::from_value(cx, value, strict, config).map(|s| cx.root_symbol(s).into())
	}
}

impl<'cx> FromValue<'cx> for JSVal {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<JSVal>
	where
		'cx: 'v,
	{
		let value = value.handle();
		AssertSameCompartment1(cx.as_ptr(), value.into());
		Ok(value.get())
	}
}

impl<'cx> FromValue<'cx> for Value<'cx> {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<Value<'cx>>
	where
		'cx: 'v,
	{
		let value = value.handle();
		AssertSameCompartment1(cx.as_ptr(), value.into());
		Ok(cx.root_value(value.get()).into())
	}
}

impl<'cx, T: FromValue<'cx>> FromValue<'cx> for Option<T> {
	type Config = T::Config;

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, config: T::Config) -> Result<Option<T>>
	where
		'cx: 'v,
	{
		if value.handle().is_null_or_undefined() {
			Ok(None)
		} else {
			Ok(Some(T::from_value(cx, value, strict, config)?))
		}
	}
}

// Copied from [rust-mozjs](https://github.com/servo/rust-mozjs/blob/master/src/conversions.rs#L619-L642)
struct ForOfIteratorGuard<'a> {
	root: &'a mut ForOfIterator,
}

impl<'a> ForOfIteratorGuard<'a> {
	fn new(cx: &Context, root: &'a mut ForOfIterator) -> Self {
		unsafe {
			root.iterator.add_to_root_stack(cx.as_ptr());
		}
		ForOfIteratorGuard { root }
	}
}

impl<'a> Drop for ForOfIteratorGuard<'a> {
	fn drop(&mut self) {
		unsafe {
			self.root.iterator.remove_from_root_stack();
		}
	}
}

impl<'cx, T: FromValue<'cx>> FromValue<'cx> for Vec<T>
where
	T::Config: Clone,
{
	type Config = T::Config;

	// Adapted from [rust-mozjs](https://github.com/servo/rust-mozjs/blob/master/src/conversions.rs#L644-L707)
	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, config: T::Config) -> Result<Vec<T>>
	where
		'cx: 'v,
	{
		if !value.handle().is_object() {
			return Err(Error::new("Expected Object", ErrorKind::Type));
		}
		let object = value.to_object(cx);
		if strict && !Array::is_array(cx, &object) {
			return Err(Error::new("Expected Array", ErrorKind::Type));
		}

		let mut iterator = ForOfIterator {
			cx_: cx.as_ptr(),
			iterator: RootedObject::new_unrooted(),
			nextMethod: RootedValue::new_unrooted(),
			index: u32::MAX, // NOT_ARRAY
		};
		let iterator = ForOfIteratorGuard::new(cx, &mut iterator);
		let iterator = &mut *iterator.root;

		if !iterator.init(value.handle().into(), ForOfIterator_NonIterableBehavior::AllowNonIterable) {
			return Err(Error::new("Failed to Initialise Iterator", ErrorKind::Type));
		}

		if iterator.iterator.ptr.is_null() {
			return Err(Error::new("Expected Iterable", ErrorKind::Type));
		}

		let mut ret = vec![];

		let mut value = Value::undefined(cx);
		loop {
			let mut done = false;
			if !iterator.next(value.handle_mut().into(), &mut done) {
				return Err(Error::new("Failed to Execute Next on Iterator", ErrorKind::Type));
			}

			if done {
				break;
			}
			ret.push(T::from_value(cx, &value, strict, config.clone())?);
		}
		Ok(ret)
	}
}

impl<'cx, T: TypedArrayElement, S: JSObjectStorage> FromValue<'cx> for TypedArray<T, S> {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<TypedArray<T, S>>
	where
		'cx: 'v,
	{
		let value = value.handle();
		if value.is_object() {
			let object = value.to_object();
			cx.root_object(object);
			TypedArray::from(object).map_err(|_| Error::new("Expected Typed Array", ErrorKind::Type))
		} else {
			Err(Error::new("Expected Object", ErrorKind::Type))
		}
	}
}
