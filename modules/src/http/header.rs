/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use hyper::header::{Entry, HeaderMap, HeaderName, HeaderValue};
use mozjs::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use mozjs::rust::HandleValue;
use mozjs::rust::MutableHandleValue;

use ion::{Array, Context, Key, Object};
use ion::types::values::from_value;

#[derive(Default)]
pub struct Headers {
	headers: HeaderMap,
}

impl Headers {
	pub fn new(headers: HeaderMap) -> Headers {
		Headers { headers }
	}

	pub fn inner(self) -> HeaderMap {
		self.headers
	}
}

impl Deref for Headers {
	type Target = HeaderMap;

	fn deref(&self) -> &Self::Target {
		&self.headers
	}
}

impl DerefMut for Headers {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.headers
	}
}

pub(crate) fn insert_header(map: &mut HeaderMap, name: HeaderName, value: HeaderValue, unique: bool) {
	match map.entry(name) {
		Entry::Occupied(mut o) => {
			if !unique {
				o.append(value);
			} else {
				o.insert(value);
			}
		}
		Entry::Vacant(v) => {
			v.insert(value);
		}
	}
}

pub(crate) fn append_to_headers(cx: Context, headers: &mut HeaderMap, obj: Object, unique: bool) {
	for key in obj.keys(cx, None) {
		let key = match key {
			Key::Int(i) => i.to_string(),
			Key::String(s) => s,
			Key::Void => continue,
		};

		if let Ok(name) = HeaderName::from_str(&key.to_lowercase()) {
			let value = obj.get(cx, &key).unwrap();
			if let Some(array) = Array::from_value(cx, value) {
				if !unique {
					for i in 0..array.len(cx) {
						if let Some(str) = array.get_as::<String>(cx, i, ()) {
							if let Ok(value) = HeaderValue::from_str(&str) {
								insert_header(headers, name.clone(), value, unique);
							}
						}
					}
				} else {
					let vec: Vec<String> = array.to_vec(cx).into_iter().filter_map(|v| from_value(cx, v, ())).collect();
					let str = vec.join(";");
					if let Ok(value) = HeaderValue::from_str(&str) {
						insert_header(headers, name.clone(), value, unique);
					}
				}
			} else if let Some(str) = from_value::<String>(cx, value, ()) {
				if let Ok(value) = HeaderValue::from_str(&str) {
					insert_header(headers, name, value, unique);
				}
			} else {
				continue;
			};
		}
	}
}

impl FromJSValConvertible for Headers {
	type Config = ();

	unsafe fn from_jsval(cx: Context, val: HandleValue, _: ()) -> Result<ConversionResult<Self>, ()> {
		let res: ConversionResult<Object> = FromJSValConvertible::from_jsval(cx, val, ())?;
		match res {
			ConversionResult::Success(obj) => {
				let headers: Option<Object> = obj.get_as(cx, "headers", ());
				let unique_headers: Option<Object> = obj.get_as(cx, "uniqueHeaders", ());

				let mut header_map = HeaderMap::new();
				if let Some(headers) = headers {
					append_to_headers(cx, &mut header_map, headers, false);
				}
				if let Some(unique_headers) = unique_headers {
					append_to_headers(cx, &mut header_map, unique_headers, true);
				}

				Ok(ConversionResult::Success(Headers { headers: header_map }))
			}
			ConversionResult::Failure(e) => Ok(ConversionResult::Failure(e)),
		}
	}
}

impl ToJSValConvertible for Headers {
	unsafe fn to_jsval(&self, cx: Context, mut rval: MutableHandleValue) {
		let mut obj = Object::new(cx);

		for key in self.headers.keys() {
			let values: Vec<_> = self.headers.get_all(key).into_iter().collect();
			if values.len() == 1 {
				obj.set_as(cx, key.as_str(), String::from(values[0].to_str().unwrap()));
			} else if values.len() > 1 {
				let values: Vec<String> = values.iter().filter_map(|v| v.to_str().ok().map(String::from)).collect();
				obj.set_as(cx, key.as_str(), values);
			}
		}

		rval.set(obj.to_value());
	}
}
