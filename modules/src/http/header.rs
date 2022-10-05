/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::str::FromStr;

use hyper::header::{HeaderMap, HeaderName, HeaderValue};
use mozjs::conversions::ToJSValConvertible;
use mozjs::jsapi::JSContext;
use mozjs::rust::MutableHandleValue;

pub use class::*;
use ion::{Array, Context, Error, Key, Object, Result};
use ion::types::values::from_value;

#[derive(FromJSVal)]
pub enum Header {
	#[ion(inherit)]
	Multiple(Vec<String>),
	#[ion(inherit)]
	Single(String),
}

#[derive(FromJSVal)]
pub enum HeadersInit {
	#[ion(inherit)]
	Existing(Headers),
	#[ion(inherit)]
	Array(Vec<Vec<String>>),
	#[ion(inherit)]
	Object(Object),
}

impl ToJSValConvertible for Header {
	unsafe fn to_jsval(&self, cx: *mut JSContext, rval: MutableHandleValue) {
		match self {
			Header::Multiple(vec) => vec.to_jsval(cx, rval),
			Header::Single(str) => str.to_jsval(cx, rval),
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
	use std::result;
	use std::str::FromStr;

	use http::header::{Entry, HeaderMap, HeaderName, HeaderValue};
	use mozjs::conversions::{ConversionResult, FromJSValConvertible};
	use mozjs::rust::HandleValue;

	use ion::{Context, Error, ErrorKind, Object, Result};
	use ion::class::class_from_jsval;

	use crate::http::header::{append_to_headers, Header, HeadersInit};

	#[derive(Clone, Default)]
	pub struct Headers {
		headers: HeaderMap,
		readonly: bool,
	}

	impl Headers {
		#[ion(internal)]
		pub fn new(headers: HeaderMap, readonly: bool) -> Headers {
			Headers { headers, readonly }
		}

		#[ion(internal)]
		pub fn inner(self) -> HeaderMap {
			self.headers
		}

		#[ion(internal)]
		pub unsafe fn from_array(vec: Vec<Vec<String>>, readonly: bool) -> Result<Headers> {
			let mut headers = HeaderMap::new();
			for mut vec in vec {
				if vec.len() != 2 {
					return Err(Error::new(
						&format!("Received Header Entry with Length {}, Expected Length 2", vec.len()),
						Some(ErrorKind::Type),
					));
				}
				let value = vec.remove(1);
				let mut name = vec.remove(0);
				name.make_ascii_lowercase();

				let name = HeaderName::from_str(&name)?;
				let value = HeaderValue::try_from(&value)?;
				headers.append(name, value);
			}
			Ok(Headers { headers, readonly })
		}

		#[ion(internal)]
		pub unsafe fn from_object(cx: Context, object: Object, readonly: bool) -> Result<Headers> {
			let mut headers = HeaderMap::new();
			append_to_headers(cx, &mut headers, object, false)?;
			Ok(Headers { headers, readonly })
		}

		#[ion(internal)]
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
		pub unsafe fn constructor(cx: Context, init: Option<HeadersInit>) -> Result<Headers> {
			match init {
				Some(HeadersInit::Existing(existing)) => {
					let headers = existing.headers;
					Ok(Headers { headers, readonly: existing.readonly })
				}
				Some(HeadersInit::Array(vec)) => Headers::from_array(vec, false),
				Some(HeadersInit::Object(object)) => Headers::from_object(cx, object, false),
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

	impl FromJSValConvertible for Headers {
		type Config = ();

		unsafe fn from_jsval(cx: Context, val: HandleValue, _: ()) -> result::Result<ConversionResult<Self>, ()> {
			class_from_jsval(cx, val)
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
