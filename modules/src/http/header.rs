/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;
use std::result;
use std::str::FromStr;

use hyper::header::{HeaderMap, HeaderName, HeaderValue};
use mozjs::conversions::{ConversionResult, FromJSValConvertible, jsstr_to_string, ToJSValConvertible};
use mozjs::rust::{HandleValue, MutableHandleValue};

pub use class::*;
use ion::{Array, Context, Error, ErrorKind, Key, Object, Result};
use ion::error::ThrowException;
use ion::types::values::from_value;

#[derive(FromJSVal)]
pub enum Header {
	#[ion(inherit)]
	Multiple(Vec<String>),
	#[ion(inherit)]
	Single(String),
}

pub struct HeadersObject {
	headers: HeaderMap,
}

pub struct HeaderEntry {
	name: String,
	value: String,
}

#[derive(FromJSVal)]
pub enum HeadersInit {
	#[ion(inherit)]
	Existing(Headers),
	#[ion(inherit)]
	Array(Vec<HeaderEntry>),
	#[ion(inherit)]
	Object(HeadersObject),
}

impl ToJSValConvertible for Header {
	unsafe fn to_jsval(&self, cx: Context, rval: MutableHandleValue) {
		match self {
			Header::Multiple(vec) => vec.to_jsval(cx, rval),
			Header::Single(str) => str.to_jsval(cx, rval),
		}
	}
}

impl FromJSValConvertible for HeaderEntry {
	type Config = ();

	unsafe fn from_jsval(cx: Context, val: HandleValue, _: ()) -> result::Result<ConversionResult<HeaderEntry>, ()> {
		if let ConversionResult::Success(array) = Array::from_jsval(cx, val, ())? {
			let length = array.len(cx);
			if length != 2 {
				Error::new(
					&format!("Received Header Entry with Length {}, Expected Length 2", length),
					Some(ErrorKind::Type),
				)
				.throw(cx);
				return Err(());
			}
			let name = jsstr_to_string(cx, array.get(cx, 0).unwrap().to_string());
			let value = jsstr_to_string(cx, array.get(cx, 1).unwrap().to_string());
			Ok(ConversionResult::Success(HeaderEntry { name, value }))
		} else {
			Error::new("Expected Array", Some(ErrorKind::Type)).throw(cx);
			Err(())
		}
	}
}

impl ToJSValConvertible for HeaderEntry {
	unsafe fn to_jsval(&self, cx: Context, mut rval: MutableHandleValue) {
		let mut array = Array::new(cx);
		array.set_as(cx, 0, &self.name);
		array.set_as(cx, 1, &self.value);
		rval.set(array.to_value());
	}
}

impl FromJSValConvertible for HeadersObject {
	type Config = ();

	unsafe fn from_jsval(cx: Context, val: HandleValue, _: ()) -> result::Result<ConversionResult<HeadersObject>, ()> {
		if let Some(object) = Object::from_value(val.get()) {
			let mut headers = HeaderMap::new();
			if let Err(err) = append_to_headers(cx, &mut headers, object, false) {
				err.throw(cx);
				return Err(());
			}
			Ok(ConversionResult::Success(HeadersObject { headers }))
		} else {
			Ok(ConversionResult::Failure(Cow::Borrowed("Headers are not an Object")))
		}
	}
}

impl HeadersInit {
	pub(crate) fn into_headers(self) -> Result<Headers> {
		unsafe {
			match self {
				HeadersInit::Existing(existing) => {
					let headers = existing.headers;
					Ok(Headers { headers, readonly: existing.readonly })
				}
				HeadersInit::Array(vec) => Headers::from_array(vec, false),
				HeadersInit::Object(object) => Ok(Headers { headers: object.headers, readonly: false }),
			}
		}
	}
}

impl Default for HeadersInit {
	fn default() -> HeadersInit {
		HeadersInit::Existing(Headers::new(HeaderMap::new(), false))
	}
}

#[js_class]
mod class {
	use std::cmp::Ordering;
	use std::ops::{Deref, DerefMut};
	use std::str::FromStr;

	use http::header::{Entry, HeaderMap, HeaderName, HeaderValue};

	use ion::{Error, Result};

	use crate::http::header::{Header, HeaderEntry, HeadersInit};

	#[derive(Clone, Default)]
	#[ion(from_jsval, to_jsval)]
	pub struct Headers {
		pub(crate) headers: HeaderMap,
		pub(crate) readonly: bool,
	}

	impl Headers {
		#[ion(skip)]
		pub fn new(headers: HeaderMap, readonly: bool) -> Headers {
			Headers { headers, readonly }
		}

		#[ion(skip)]
		pub fn inner(self) -> HeaderMap {
			self.headers
		}

		#[ion(skip)]
		pub unsafe fn from_array(vec: Vec<HeaderEntry>, readonly: bool) -> Result<Headers> {
			let mut headers = HeaderMap::new();
			for entry in vec {
				let mut name = entry.name;
				let value = entry.value;
				name.make_ascii_lowercase();

				let name = HeaderName::from_str(&name)?;
				let value = HeaderValue::try_from(&value)?;
				headers.append(name, value);
			}
			Ok(Headers { headers, readonly })
		}

		#[ion(skip)]
		pub fn get_internal(&self, name: &HeaderName) -> Result<Option<Header>> {
			let values: Vec<_> = self.headers.get_all(name).into_iter().collect();
			match values.len().cmp(&1) {
				Ordering::Less => Ok(None),
				Ordering::Equal => Ok(Some(Header::Single(String::from(values[0].to_str()?)))),
				Ordering::Greater => {
					let values: Vec<String> = values.iter().map(|v| Ok(String::from(v.to_str()?))).collect::<Result<_>>()?;
					Ok(Some(Header::Multiple(values)))
				}
			}
		}

		#[ion(constructor)]
		pub fn constructor(init: Option<HeadersInit>) -> Result<Headers> {
			match init {
				Some(init) => init.into_headers(),
				None => Ok(Headers::default()),
			}
		}

		pub fn append(&mut self, name: String, value: String) -> Result<()> {
			if !self.readonly {
				let name = HeaderName::from_str(&name.to_lowercase())?;
				let value = HeaderValue::from_str(&value)?;
				self.headers.append(name, value);
				Ok(())
			} else {
				Err(Error::new("Cannot Modify Readonly Headers", None))
			}
		}

		pub fn delete(&mut self, name: String) -> Result<bool> {
			if !self.readonly {
				let name = HeaderName::from_str(&name.to_lowercase())?;
				match self.headers.entry(name) {
					Entry::Occupied(o) => {
						o.remove_entry_mult();
						Ok(true)
					}
					Entry::Vacant(_) => Ok(false),
				}
			} else {
				Err(Error::new("Cannot Modify Readonly Headers", None))
			}
		}

		pub fn get(&self, name: String) -> Result<Option<Header>> {
			let name = HeaderName::from_str(&name.to_lowercase())?;
			self.get_internal(&name)
		}

		pub fn has(&self, name: String) -> Result<bool> {
			let name = HeaderName::from_str(&name.to_lowercase())?;
			Ok(self.headers.contains_key(name))
		}

		pub fn set(&mut self, name: String, value: String) -> Result<()> {
			if !self.readonly {
				let name = HeaderName::from_str(&name.to_lowercase())?;
				let value = HeaderValue::from_str(&value)?;
				self.headers.insert(name, value);
				Ok(())
			} else {
				Err(Error::new("Cannot Modify Readonly Headers", None))
			}
		}
	}

	impl Deref for Headers {
		type Target = HeaderMap;

		fn deref(&self) -> &HeaderMap {
			&self.headers
		}
	}

	impl DerefMut for Headers {
		fn deref_mut(&mut self) -> &mut HeaderMap {
			&mut self.headers
		}
	}
}

fn append_to_headers(cx: Context, headers: &mut HeaderMap, obj: Object, unique: bool) -> Result<()> {
	for key in obj.keys(cx, None) {
		let key = match key {
			Key::Int(i) => i.to_string(),
			Key::String(s) => s,
			Key::Void => continue,
		};

		let name = HeaderName::from_str(&key.to_lowercase())?;
		let value = obj.get(cx, &key).unwrap();
		if let Some(array) = Array::from_value(cx, value) {
			if !unique {
				for i in 0..array.len(cx) {
					if let Some(str) = array.get_as::<String>(cx, i, ()) {
						let value = HeaderValue::from_str(&str)?;
						headers.insert(name.clone(), value);
					}
				}
			} else {
				let vec: Vec<String> = array
					.to_vec(cx)
					.into_iter()
					.map(|v| from_value(cx, v, ()).ok_or_else(|| Error::new("Could not Convert Header Value to String", None)))
					.collect::<Result<_>>()?;
				let str = vec.join(";");
				let value = HeaderValue::from_str(&str)?;
				headers.insert(name, value);
			}
		} else if let Some(str) = from_value::<String>(cx, value, ()) {
			let value = HeaderValue::from_str(&str)?;
			headers.insert(name, value);
		} else {
			return Err(Error::new("Could not convert value to Header Value", None));
		};
	}
	Ok(())
}
