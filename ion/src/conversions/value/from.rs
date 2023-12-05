/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::conversions::{ConversionResult, FromJSValConvertible};
pub use mozjs::conversions::ConversionBehavior;
use mozjs::jsapi::{
	AssertSameCompartment, AssertSameCompartment1, ForOfIterator, ForOfIterator_NonIterableBehavior, JSFunction,
	JSObject, JSString, RootedObject, RootedValue,
};
use mozjs::jsapi::Symbol as JSSymbol;
use mozjs::jsval::{JSVal, UndefinedValue};
use mozjs::rust::{ToBoolean, ToNumber, ToString};
use mozjs::typedarray::{JSObjectStorage, TypedArray, TypedArrayElement};

use crate::{
	Array, Context, Date, Error, ErrorKind, Exception, Function, Object, Promise, Result, StringRef, Symbol, Value,
};
use crate::objects::RegExp;
use crate::string::byte::{BytePredicate, ByteString};

/// Represents types that can be converted to from [JavaScript Values](Value).
pub trait FromValue: Sized {
	type Config;

	/// Converts `value` to the desired type.
	/// `strict` and `config` determine the strictness of the conversion and specify additional conversion constraints respectively.
	/// Returns [Err] with the [error](Error) if conversion fails.
	fn from_value(cx: &Context, value: &Value, strict: bool, config: Self::Config) -> Result<Self>;
}

impl FromValue for bool {
	type Config = ();

	fn from_value(_: &Context, value: &Value, strict: bool, _: ()) -> Result<bool> {
		let value = value.handle();
		if value.is_boolean() {
			return Ok(value.to_boolean());
		}

		if strict {
			Err(Error::new("Expected Boolean in Strict Conversion", ErrorKind::Type))
		} else {
			Ok(unsafe { ToBoolean(value) })
		}
	}
}

macro_rules! impl_from_value_for_integer {
	($ty:ty) => {
		impl FromValue for $ty {
			type Config = ConversionBehavior;

			fn from_value(cx: &Context, value: &Value, strict: bool, config: ConversionBehavior) -> Result<$ty> {
				let value = value.handle();
				if strict && !value.is_number() {
					return Err(Error::new(
						"Expected Number in Strict Conversion",
						ErrorKind::Type,
					));
				}

				match unsafe { <$ty>::from_jsval(cx.as_ptr(), value, config) } {
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

impl FromValue for f32 {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, strict: bool, _: ()) -> Result<f32> {
		f64::from_value(cx, value, strict, ()).map(|float| float as f32)
	}
}

impl FromValue for f64 {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, strict: bool, _: ()) -> Result<f64> {
		let value = value.handle();
		if strict && !value.is_number() {
			return Err(Error::new("Expected Number in Strict Conversion", ErrorKind::Type));
		}

		let number = unsafe { ToNumber(cx.as_ptr(), value) };
		number.map_err(|_| Error::new("Unable to Convert Value to Number", ErrorKind::Type))
	}
}

impl FromValue for *mut JSString {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, strict: bool, _: ()) -> Result<*mut JSString> {
		let value = value.handle();
		if strict && !value.is_string() {
			return Err(Error::new("Expected String in Strict Conversion", ErrorKind::Type));
		}
		Ok(unsafe { ToString(cx.as_ptr(), value) })
	}
}

impl FromValue for crate::String {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, strict: bool, config: ()) -> Result<crate::String> {
		<*mut JSString>::from_value(cx, value, strict, config).map(|str| crate::String::from(cx.root(str)))
	}
}

impl<T: BytePredicate> FromValue for ByteString<T> {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, strict: bool, config: ()) -> Result<ByteString<T>> {
		const INVALID_CHARACTERS: &str = "ByteString contains invalid characters";
		let string = crate::String::from_value(cx, value, strict, config)?;
		match string.as_ref(cx) {
			StringRef::Latin1(bstr) => {
				ByteString::from(bstr.to_vec()).ok_or_else(|| Error::new(INVALID_CHARACTERS, ErrorKind::Type))
			}
			StringRef::Utf16(wstr) => {
				let bytes = wstr
					.as_bytes()
					.chunks_exact(2)
					.map(|chunk| {
						let codepoint = u16::from_ne_bytes([chunk[0], chunk[1]]);
						u8::try_from(codepoint).map_err(|_| Error::new(INVALID_CHARACTERS, ErrorKind::Type))
					})
					.collect::<Result<Vec<_>>>()?;
				ByteString::from(bytes).ok_or_else(|| Error::new(INVALID_CHARACTERS, ErrorKind::Type))
			}
		}
	}
}

impl FromValue for String {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, strict: bool, config: ()) -> Result<String> {
		crate::String::from_value(cx, value, strict, config).map(|s| s.to_owned(cx))
	}
}

impl FromValue for *mut JSObject {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<*mut JSObject> {
		let value = value.handle();
		if !value.is_object() {
			return Err(Error::new("Expected Object", ErrorKind::Type));
		}
		let object = value.to_object();
		unsafe {
			AssertSameCompartment(cx.as_ptr(), object);
		}

		Ok(object)
	}
}

impl FromValue for Object {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<Object> {
		if !value.handle().is_object() {
			return Err(Error::new("Expected Object", ErrorKind::Type));
		}
		let object = value.to_object(cx);
		unsafe {
			AssertSameCompartment(cx.as_ptr(), object.handle().get());
		}

		Ok(object)
	}
}

impl FromValue for Array {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<Array> {
		if !value.handle().is_object() {
			return Err(Error::new("Expected Array", ErrorKind::Type));
		}

		let object = value.to_object(cx).into_root();
		if let Some(array) = Array::from(cx, object) {
			unsafe {
				AssertSameCompartment(cx.as_ptr(), array.handle().get());
			}
			Ok(array)
		} else {
			Err(Error::new("Expected Array", ErrorKind::Type))
		}
	}
}

impl FromValue for Date {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<Date> {
		if !value.handle().is_object() {
			return Err(Error::new("Expected Date", ErrorKind::Type));
		}

		let object = value.to_object(cx).into_root();
		if let Some(date) = Date::from(cx, object) {
			unsafe {
				AssertSameCompartment(cx.as_ptr(), date.get());
			}
			Ok(date)
		} else {
			Err(Error::new("Expected Date", ErrorKind::Type))
		}
	}
}

impl FromValue for Promise {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<Promise> {
		if !value.handle().is_object() {
			return Err(Error::new("Expected Promise", ErrorKind::Type));
		}

		let object = value.to_object(cx).into_root();
		if let Some(promise) = Promise::from(object) {
			unsafe {
				AssertSameCompartment(cx.as_ptr(), promise.get());
			}
			Ok(promise)
		} else {
			Err(Error::new("Expected Promise", ErrorKind::Type))
		}
	}
}

impl FromValue for RegExp {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<RegExp> {
		if !value.handle().is_object() {
			return Err(Error::new("Expected RegExp", ErrorKind::Type));
		}

		let object = value.to_object(cx).into_root();
		if let Some(regexp) = RegExp::from(cx, object) {
			unsafe {
				AssertSameCompartment(cx.as_ptr(), regexp.get());
			}
			Ok(regexp)
		} else {
			Err(Error::new("Expected RegExp", ErrorKind::Type))
		}
	}
}

impl FromValue for *mut JSFunction {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, strict: bool, config: ()) -> Result<*mut JSFunction> {
		Function::from_value(cx, value, strict, config).map(|f| f.get())
	}
}

impl FromValue for Function {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<Function> {
		if !value.handle().is_object() {
			return Err(Error::new("Expected Function", ErrorKind::Type));
		}

		let function_obj = value.to_object(cx);
		if let Some(function) = Function::from_object(cx, &function_obj) {
			unsafe {
				AssertSameCompartment(cx.as_ptr(), function_obj.handle().get());
			}
			Ok(function)
		} else {
			Err(Error::new("Expected Function", ErrorKind::Type))
		}
	}
}

impl FromValue for *mut JSSymbol {
	type Config = ();

	fn from_value(_: &Context, value: &Value, _: bool, _: ()) -> Result<*mut JSSymbol> {
		let value = value.handle();
		if value.is_symbol() {
			Ok(value.to_symbol())
		} else {
			Err(Error::new("Expected Symbol", ErrorKind::Type))
		}
	}
}

impl FromValue for Symbol {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, strict: bool, config: Self::Config) -> Result<Symbol> {
		<*mut JSSymbol>::from_value(cx, value, strict, config).map(|s| cx.root(s).into())
	}
}

impl FromValue for JSVal {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<JSVal> {
		let value = value.handle();
		unsafe {
			AssertSameCompartment1(cx.as_ptr(), value.into());
		}
		Ok(value.get())
	}
}

impl FromValue for Value {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<Value> {
		let value = value.handle();
		unsafe {
			AssertSameCompartment1(cx.as_ptr(), value.into());
		}
		Ok(cx.root(value.get()).into())
	}
}

impl<T: FromValue> FromValue for Option<T> {
	type Config = T::Config;

	fn from_value(cx: &Context, value: &Value, strict: bool, config: T::Config) -> Result<Option<T>> {
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
		cx.root(root.iterator.ptr);
		ForOfIteratorGuard { root }
	}
}

impl<T: FromValue> FromValue for Vec<T>
where
	T::Config: Clone,
{
	type Config = T::Config;

	// Adapted from [rust-mozjs](https://github.com/servo/rust-mozjs/blob/master/src/conversions.rs#L644-L707)
	fn from_value(cx: &Context, value: &Value, strict: bool, config: T::Config) -> Result<Vec<T>> {
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

		let init = unsafe {
			iterator.init(
				value.handle().into(),
				ForOfIterator_NonIterableBehavior::AllowNonIterable,
			)
		};
		if !init {
			return Err(Error::new("Failed to Initialise Iterator", ErrorKind::Type));
		}

		if iterator.iterator.ptr.is_null() {
			return Err(Error::new("Expected Iterable", ErrorKind::Type));
		}

		let mut ret = vec![];

		rooted!(in(cx.as_ptr()) let mut value = UndefinedValue());
		loop {
			let mut done = false;
			if unsafe { !iterator.next(value.handle_mut().into(), &mut done) } {
				return Err(Error::new("Failed to Execute Next on Iterator", ErrorKind::Type));
			}

			if done {
				break;
			}
			ret.push(T::from_value(
				cx,
				&Value::from(cx.root(value.get())),
				strict,
				config.clone(),
			)?);
		}
		Ok(ret)
	}
}

impl<T: TypedArrayElement, S: JSObjectStorage> FromValue for TypedArray<T, S> {
	type Config = ();

	fn from_value(cx: &Context, value: &Value, _: bool, _: ()) -> Result<TypedArray<T, S>> {
		let value = value.handle();
		if value.is_object() {
			let object = value.to_object();
			cx.root(object);
			TypedArray::from(object).map_err(|_| Error::new("Expected Typed Array", ErrorKind::Type))
		} else {
			Err(Error::new("Expected Object", ErrorKind::Type))
		}
	}
}
