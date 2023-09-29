/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::str::FromStr;

use hyper::header::{HeaderMap, HeaderName, HeaderValue};

pub use class::*;
use ion::{Array, Context, Error, ErrorKind, Object, OwnedKey, Result, Value};
use ion::conversions::{FromValue, ToValue};

#[derive(FromValue)]
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

#[derive(FromValue)]
pub enum HeadersInit {
	#[ion(inherit)]
	Existing(Headers),
	#[ion(inherit)]
	Array(Vec<HeaderEntry>),
	#[ion(inherit)]
	Object(HeadersObject),
}

impl ToValue<'_> for Header {
	unsafe fn to_value(&self, cx: &Context, value: &mut Value) {
		match self {
			Header::Multiple(vec) => vec.to_value(cx, value),
			Header::Single(str) => str.to_value(cx, value),
		}
	}
}

impl<'cx> FromValue<'cx> for HeaderEntry {
	type Config = ();
	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<HeaderEntry>
	where
		'cx: 'v,
	{
		let vec = Vec::<String>::from_value(cx, value, false, ())?;
		if vec.len() != 2 {
			return Err(Error::new(
				&format!("Received Header Entry with Length {}, Expected Length 2", vec.len()),
				ErrorKind::Type,
			));
		}
		Ok(HeaderEntry {
			name: vec[0].clone(),
			value: vec[1].clone(),
		})
	}
}

impl ToValue<'_> for HeaderEntry {
	unsafe fn to_value(&self, cx: &Context, value: &mut Value) {
		let mut array = Array::new(cx);
		array.set_as(cx, 0, &self.name);
		array.set_as(cx, 1, &self.value);
		array.to_value(cx, value);
	}
}

impl<'cx> FromValue<'cx> for HeadersObject {
	type Config = ();

	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<HeadersObject>
	where
		'cx: 'v,
	{
		let object = Object::from_value(cx, value, true, ())?;
		let mut headers = HeaderMap::new();
		append_to_headers(cx, &mut headers, object, false)?;
		Ok(HeadersObject { headers })
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

	use crate::globals::fetch::header::{Header, HeaderEntry, HeadersInit};

	#[derive(Clone, Default)]
	#[ion(from_value, to_value)]
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

fn append_to_headers<'cx: 'o, 'o>(cx: &'cx Context, headers: &mut HeaderMap, obj: Object<'o>, unique: bool) -> Result<()> {
	for key in obj.keys(cx, None).map(|key| key.to_owned_key(cx)) {
		let key = match key {
			OwnedKey::Int(i) => i.to_string(),
			OwnedKey::String(s) => s,
			_ => continue,
		};

		let name = HeaderName::from_str(&key.to_lowercase())?;
		let value = obj.get(cx, &key).unwrap();
		if let Ok(array) = unsafe { Array::from_value(cx, &value, false, ()) } {
			if !unique {
				for i in 0..array.len(cx) {
					if let Some(str) = array.get_as::<String>(cx, i, false, ()) {
						let value = HeaderValue::from_str(&str)?;
						headers.insert(name.clone(), value);
					}
				}
			} else {
				let vec: Vec<_> = array
					.to_vec(cx)
					.into_iter()
					.map(|v| unsafe { String::from_value(cx, &v, false, ()) })
					.collect::<Result<_>>()?;
				let str = vec.join(";");
				let value = HeaderValue::from_str(&str)?;
				headers.insert(name, value);
			}
		} else if let Ok(str) = unsafe { String::from_value(cx, &value, false, ()) } {
			let value = HeaderValue::from_str(&str)?;
			headers.insert(name, value);
		} else {
			return Err(Error::new("Could not convert value to Header Value", ErrorKind::Type));
		};
	}
	Ok(())
}
