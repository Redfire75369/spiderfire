/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str;
use std::str::FromStr;

use http::header::{
	ACCEPT, ACCEPT_CHARSET, ACCEPT_ENCODING, ACCEPT_LANGUAGE, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, CONNECTION,
	CONTENT_LANGUAGE, CONTENT_LENGTH, CONTENT_TYPE, COOKIE, DATE, DNT, Entry, EXPECT, HOST, ORIGIN, RANGE, REFERER, SET_COOKIE, TE, TRAILER,
	TRANSFER_ENCODING, UPGRADE, VIA,
};
use hyper::header::{HeaderMap, HeaderName, HeaderValue};
use mime::{APPLICATION, FORM_DATA, Mime, MULTIPART, PLAIN, TEXT, WWW_FORM_URLENCODED};
use mozjs::gc::Traceable;
use mozjs::jsapi::{Heap, JSObject, JSTracer};

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

impl Display for Header {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Header::Multiple(vec) => f.write_str(&vec.join(", ")),
			Header::Single(str) => f.write_str(str),
		}
	}
}

impl ToValue<'_> for Header {
	fn to_value(&self, cx: &Context, value: &mut Value) {
		self.to_string().to_value(cx, value)
	}
}

pub struct HeadersObject(HeaderMap);

impl<'cx> FromValue<'cx> for HeadersObject {
	type Config = ();

	fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<HeadersObject>
	where
		'cx: 'v,
	{
		let object = Object::from_value(cx, value, true, ())?;
		let mut headers = HeaderMap::new();
		append_to_headers(cx, &mut headers, object)?;
		Ok(HeadersObject(headers))
	}
}

pub struct HeaderEntry {
	name: String,
	value: String,
}

impl<'cx> FromValue<'cx> for HeaderEntry {
	type Config = ();
	fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<HeaderEntry>
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
	fn to_value(&self, cx: &Context, value: &mut Value) {
		let mut array = Array::new(cx);
		array.set_as(cx, 0, &self.name);
		array.set_as(cx, 1, &self.value);
		array.to_value(cx, value);
	}
}

#[derive(Default, FromValue)]
pub enum HeadersInit {
	#[ion(inherit)]
	Existing(Headers),
	#[ion(inherit)]
	Array(Vec<HeaderEntry>),
	#[ion(inherit)]
	Object(HeadersObject),
	#[default]
	#[ion(skip)]
	Empty,
}

impl HeadersInit {
	pub(crate) fn into_headers(self, mut headers: HeadersInner, kind: HeadersKind) -> Result<Headers> {
		match self {
			HeadersInit::Existing(existing) => {
				headers
					.as_mut()
					.extend(existing.headers.as_ref().into_iter().map(|(name, value)| (name.clone(), value.clone())));
				Ok(Headers { headers, kind })
			}
			HeadersInit::Array(vec) => Headers::from_array(vec, headers, kind),
			HeadersInit::Object(object) => {
				let mut name = None;
				for (nm, value) in object.0 {
					if let nm @ Some(_) = nm {
						name = nm;
					}
					append_header(headers.as_mut(), name.clone().unwrap(), value, kind)?;
				}
				Ok(Headers { headers, kind })
			}
			HeadersInit::Empty => Ok(Headers { headers, kind }),
		}
	}
}

#[derive(Debug)]
pub(crate) enum HeadersInner {
	Owned(HeaderMap),
	MutRef(*mut HeaderMap, Box<Heap<*mut JSObject>>),
}

impl HeadersInner {
	pub fn as_ref(&self) -> &HeaderMap {
		match self {
			HeadersInner::Owned(map) => map,
			HeadersInner::MutRef(map, _) => unsafe { &**map },
		}
	}

	pub fn as_mut(&mut self) -> &mut HeaderMap {
		match self {
			HeadersInner::Owned(map) => map,
			HeadersInner::MutRef(map, _) => unsafe { &mut **map },
		}
	}
}

impl Clone for HeadersInner {
	fn clone(&self) -> HeadersInner {
		match self {
			HeadersInner::Owned(map) => HeadersInner::Owned(map.clone()),
			HeadersInner::MutRef(map, _) => HeadersInner::Owned(unsafe { (**map).clone() }),
		}
	}
}

impl Default for HeadersInner {
	fn default() -> HeadersInner {
		HeadersInner::Owned(HeaderMap::new())
	}
}

unsafe impl Traceable for HeadersInner {
	unsafe fn trace(&self, trc: *mut JSTracer) {
		if let HeadersInner::MutRef(_, source) = self {
			unsafe {
				source.trace(trc);
			}
		}
	}
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum HeadersKind {
	Immutable,
	Request,
	RequestNoCors,
	Response,
	#[default]
	None,
}

#[js_class]
mod class {
	use std::ops::{Deref, DerefMut};
	use std::str::FromStr;
	use std::vec;

	use http::header::{Entry, HeaderMap, HeaderName, HeaderValue, RANGE, SET_COOKIE};
	use mozjs::gc::Traceable;
	use mozjs::jsapi::JSTracer;

	use ion::{ClassDefinition, Context, Error, JSIterator, Object, Result, Value};
	use ion::conversions::ToValue;
	use ion::symbol::WellKnownSymbolCode;

	use crate::globals::fetch::header::{
		append_header, get_header, Header, HeaderEntry, HeadersInit, HeadersInner, HeadersKind, NO_CORS_SAFELISTED_REQUEST_HEADERS,
		remove_privileged_no_cors_headers, validate_header, validate_no_cors_safelisted_request_header,
	};

	#[derive(Clone, Default)]
	#[ion(from_value, to_value)]
	pub struct Headers {
		pub(crate) headers: HeadersInner,
		pub(crate) kind: HeadersKind,
	}

	impl Headers {
		pub(crate) fn from_array(vec: Vec<HeaderEntry>, mut headers: HeadersInner, kind: HeadersKind) -> Result<Headers> {
			for entry in vec {
				let mut name = entry.name;
				let value = entry.value;
				name.make_ascii_lowercase();

				let name = HeaderName::from_str(&name)?;
				let value = HeaderValue::try_from(&value)?;
				append_header(headers.as_mut(), name, value, kind)?;
			}
			Ok(Headers { headers, kind })
		}

		#[ion(constructor)]
		pub fn constructor(init: Option<HeadersInit>) -> Result<Headers> {
			init.unwrap_or_default().into_headers(HeadersInner::default(), HeadersKind::None)
		}

		pub fn append(&mut self, name: String, value: String) -> Result<()> {
			if self.kind != HeadersKind::Immutable {
				let name = HeaderName::from_str(&name.to_lowercase())?;
				let value = HeaderValue::from_str(&value)?;
				self.headers.as_mut().append(name, value);
				Ok(())
			} else {
				Err(Error::new("Cannot Modify Readonly Headers", None))
			}
		}

		pub fn delete(&mut self, name: String) -> Result<()> {
			let name = HeaderName::from_str(&name.to_lowercase())?;
			if !validate_header(&name, &HeaderValue::from_static(""), self.kind)? {
				return Ok(());
			}

			if self.kind == HeadersKind::RequestNoCors && !NO_CORS_SAFELISTED_REQUEST_HEADERS.contains(&name) && name != RANGE {
				return Ok(());
			}

			match self.headers.as_mut().entry(name) {
				Entry::Occupied(o) => {
					o.remove_entry_mult();
				}
				Entry::Vacant(_) => (),
			}
			remove_privileged_no_cors_headers(self.headers.as_mut(), self.kind);
			Ok(())
		}

		pub fn get(&self, name: String) -> Result<Option<Header>> {
			let name = HeaderName::from_str(&name.to_lowercase())?;
			Ok(get_header(self.headers.as_ref(), &name))
		}

		pub fn get_set_cookie(&self) -> Vec<String> {
			let header = get_header(self.headers.as_ref(), &SET_COOKIE);
			header.map_or_else(Vec::new, |header| match header {
				Header::Multiple(vec) => vec,
				Header::Single(str) => vec![str],
			})
		}

		pub fn has(&self, name: String) -> Result<bool> {
			let name = HeaderName::from_str(&name.to_lowercase())?;
			Ok(self.headers.as_ref().contains_key(name))
		}

		pub fn set(&mut self, name: String, value: String) -> Result<()> {
			let name = HeaderName::from_str(&name.to_lowercase())?;
			let value = HeaderValue::from_str(&value)?;
			if !validate_header(&name, &HeaderValue::from_static(""), self.kind)? {
				return Ok(());
			}
			if self.kind == HeadersKind::RequestNoCors && !validate_no_cors_safelisted_request_header(self.headers.as_mut(), &name, &value) {
				return Ok(());
			}
			self.headers.as_mut().insert(name, value);
			remove_privileged_no_cors_headers(self.headers.as_mut(), self.kind);
			Ok(())
		}

		#[ion(name = WellKnownSymbolCode::Iterator)]
		pub fn iterator<'cx: 'o, 'o>(cx: &'cx Context, #[ion(this)] this: &Object<'o>) -> ion::Iterator {
			let thisv = this.as_value(cx);

			let self_ = Headers::get_private(this);
			let cookies: Vec<_> = self_.headers.as_ref().get_all(&SET_COOKIE).iter().map(HeaderValue::clone).collect();

			let mut keys: Vec<_> = self_.headers.as_ref().keys().map(|name| name.as_str().to_ascii_lowercase()).collect();
			keys.reserve(cookies.len() - 1);
			for _ in 0..(cookies.len() - 1) {
				keys.push(String::from(SET_COOKIE.as_str()));
			}
			keys.sort();

			ion::Iterator::new(
				HeadersIterator {
					keys: keys.into_iter(),
					cookies: cookies.into_iter(),
				},
				&thisv,
			)
		}
	}

	pub struct HeadersIterator {
		keys: vec::IntoIter<String>,
		cookies: vec::IntoIter<HeaderValue>,
	}

	impl JSIterator for HeadersIterator {
		fn next_value<'cx>(&mut self, cx: &'cx Context, private: &Value<'cx>) -> Option<Value<'cx>> {
			let object = private.to_object(cx);
			let headers = Headers::get_private(&object);
			let key = self.keys.next();
			key.and_then(|key| {
				if key == SET_COOKIE.as_str() {
					self.cookies.next().map(|value| [key.as_str(), value.to_str().unwrap()].as_value(cx))
				} else {
					get_header(headers.headers.as_ref(), &HeaderName::from_bytes(key.as_bytes()).unwrap())
						.map(|value| [key.as_str(), &value.to_string()].as_value(cx))
				}
			})
		}
	}

	impl Deref for Headers {
		type Target = HeaderMap;

		fn deref(&self) -> &HeaderMap {
			self.headers.as_ref()
		}
	}

	impl DerefMut for Headers {
		fn deref_mut(&mut self) -> &mut HeaderMap {
			self.headers.as_mut()
		}
	}

	unsafe impl Traceable for Headers {
		unsafe fn trace(&self, trc: *mut JSTracer) {
			unsafe {
				self.headers.trace(trc);
			}
		}
	}
}

const COOKIE2: HeaderName = HeaderName::from_static("cookie2");
const SET_COOKIE2: HeaderName = HeaderName::from_static("set-cookie2");
const KEEP_ALIVE: HeaderName = HeaderName::from_static("keep-alive");

const X_HTTP_METHOD: HeaderName = HeaderName::from_static("x-http-method");
const X_HTTP_METHOD_OVERRIDE: HeaderName = HeaderName::from_static("x-http-method-override");
const X_METHOD_OVERRIDE: HeaderName = HeaderName::from_static("x-method-override");

static FORBIDDEN_REQUEST_HEADERS: [HeaderName; 21] = [
	ACCEPT_CHARSET,
	ACCEPT_ENCODING,
	ACCESS_CONTROL_ALLOW_HEADERS,
	ACCESS_CONTROL_ALLOW_METHODS,
	CONNECTION,
	CONTENT_LENGTH,
	COOKIE,
	COOKIE2,
	DATE,
	DNT,
	EXPECT,
	HOST,
	KEEP_ALIVE,
	ORIGIN,
	REFERER,
	SET_COOKIE,
	TE,
	TRAILER,
	TRANSFER_ENCODING,
	UPGRADE,
	VIA,
];

static FORBIDDEN_REQUEST_HEADER_METHODS: [HeaderName; 3] = [X_HTTP_METHOD, X_HTTP_METHOD_OVERRIDE, X_METHOD_OVERRIDE];
static FORBIDDEN_RESPONSE_HEADERS: [HeaderName; 2] = [SET_COOKIE, SET_COOKIE2];

static NO_CORS_SAFELISTED_REQUEST_HEADERS: [HeaderName; 4] = [ACCEPT, ACCEPT_LANGUAGE, CONTENT_LANGUAGE, CONTENT_TYPE];

fn validate_header(name: &HeaderName, value: &HeaderValue, kind: HeadersKind) -> Result<bool> {
	if kind == HeadersKind::Immutable {
		return Err(Error::new("Headers cannot be modified", ErrorKind::Type));
	}

	if FORBIDDEN_REQUEST_HEADERS.contains(&name) {
		return Ok(false);
	}
	if name.as_str().starts_with("proxy-") || name.as_str().starts_with("sec-") {
		return Ok(false);
	}
	if FORBIDDEN_REQUEST_HEADER_METHODS.contains(&name) {
		let value = split_value(&value);
		if value.iter().any(|v| v == "CONNECT" || v == "TRACE" || v == "TRACK") {
			return Ok(false);
		}
	}

	if FORBIDDEN_RESPONSE_HEADERS.contains(&name) {
		return Ok(false);
	}

	Ok(true)
}

fn validate_no_cors_safelisted_request_header(headers: &mut HeaderMap, name: &HeaderName, value: &HeaderValue) -> bool {
	if !NO_CORS_SAFELISTED_REQUEST_HEADERS.contains(&name) {
		return false;
	}

	let temp = get_header(headers, &name);
	let str = value.to_str().unwrap();
	let temp = match temp {
		Some(temp) => format!("{}, {}", temp, str),
		None => String::from(str),
	};
	if temp.len() > 128 {
		return false;
	}

	let unsafe_header_byte = temp.as_bytes().iter().any(|b| {
		(*b < b' ' && *b != b'\t')
			|| matches!(
				b,
				b'"' | b'(' | b')' | b':' | b'<' | b'>' | b'?' | b'@' | b'[' | b']' | b'{' | b'}' | 0x7F
			)
	});
	if name == ACCEPT {
		if unsafe_header_byte {
			return false;
		}
	} else if name == ACCEPT_LANGUAGE || name == CONTENT_LANGUAGE {
		let cond = temp
			.as_bytes()
			.iter()
			.all(|b| matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b' ' | b'*' | b',' | b'-' | b'.' | b';' | b'='));
		if !cond {
			return false;
		}
	} else if name == CONTENT_TYPE {
		if unsafe_header_byte {
			return false;
		}
		let mime = Mime::from_str(str);
		match mime {
			Ok(mime) => {
				if !(mime.type_() == APPLICATION && mime.subtype() == WWW_FORM_URLENCODED)
					&& !(mime.type_() == MULTIPART && mime.subtype() == FORM_DATA)
					&& !(mime.type_() == TEXT && mime.subtype() == PLAIN)
				{
					return false;
				}
			}
			Err(_) => return false,
		}
	} else if name == RANGE {
		if !str.starts_with("bytes=") {
			return false;
		}
		let str = &str[5..];
		let digit = str.char_indices().find_map(|(i, c)| matches!(c, '0'..='9').then(|| i + 1));
		let digit = digit.unwrap_or_default();
		let start = str[0..digit].parse::<usize>().ok();
		if str.as_bytes()[digit] != b'-' {
			return false;
		}

		let str = &str[digit..];
		let digit = str.char_indices().find_map(|(i, c)| matches!(c, '0'..='9').then(|| i + 1));
		let digit = digit.unwrap_or_default();
		let end = str[0..digit].parse().ok();
		if digit != str.len() {
			return false;
		}
		match (start, end) {
			(None, _) => return false,
			(Some(start), Some(end)) if start > end => return false,
			_ => (),
		}
	} else {
		return false;
	}

	true
}

fn append_header(headers: &mut HeaderMap, name: HeaderName, value: HeaderValue, kind: HeadersKind) -> Result<()> {
	if !validate_header(&name, &value, kind)? {
		return Ok(());
	}

	if kind == HeadersKind::RequestNoCors && !validate_no_cors_safelisted_request_header(headers, &name, &value) {
		return Ok(());
	}

	headers.append(name, value);
	remove_privileged_no_cors_headers(headers, kind);
	Ok(())
}

fn remove_privileged_no_cors_headers(headers: &mut HeaderMap, kind: HeadersKind) {
	if kind == HeadersKind::RequestNoCors {
		match headers.entry(RANGE) {
			Entry::Occupied(o) => {
				o.remove_entry_mult();
			}
			Entry::Vacant(_) => (),
		}
	}
}

fn append_to_headers(cx: &Context, headers: &mut HeaderMap, obj: Object) -> Result<()> {
	for key in obj.keys(cx, None).map(|key| key.to_owned_key(cx)) {
		let key = match key {
			OwnedKey::Int(i) => i.to_string(),
			OwnedKey::String(s) => s,
			_ => continue,
		};

		let name = HeaderName::from_str(&key.to_lowercase())?;
		let value = obj.get(cx, &key).unwrap();
		if let Ok(array) = Array::from_value(cx, &value, false, ()) {
			let vec: Vec<_> = array
				.to_vec(cx)
				.into_iter()
				.map(|v| String::from_value(cx, &v, false, ()))
				.collect::<Result<_>>()?;
			let str = vec.join(", ");
			let value = HeaderValue::from_str(&str)?;
			headers.insert(name, value);
		} else if let Ok(str) = String::from_value(cx, &value, false, ()) {
			let value = HeaderValue::from_str(&str)?;
			headers.insert(name, value);
		} else {
			return Err(Error::new("Could not convert value to Header Value", ErrorKind::Type));
		};
	}
	Ok(())
}

pub fn get_header(headers: &HeaderMap, name: &HeaderName) -> Option<Header> {
	let split = headers.get_all(name).into_iter().map(split_value);
	let mut values = Vec::with_capacity(split.size_hint().0);
	for value in split {
		values.extend(value);
	}
	match values.len().cmp(&1) {
		Ordering::Less => None,
		Ordering::Equal => Some(Header::Single(values.pop().unwrap())),
		Ordering::Greater => Some(Header::Multiple(values)),
	}
}

fn split_value(value: &HeaderValue) -> Vec<String> {
	let mut quoted = false;
	let mut escaped = false;
	let mut result = vec![String::new()];

	for char in str::from_utf8(value.as_bytes()).unwrap().chars() {
		let len = result.len();
		if char == '"' && !escaped {
			quoted = !quoted;
		} else if char == ',' && !quoted {
			let str = &mut result[len - 1];
			*str = String::from(str.trim());
			result.push(String::new());
		} else {
			result[len - 1].push(char);
		}
		escaped = char == '\\';
	}
	result
}
