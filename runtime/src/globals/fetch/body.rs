/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};

use bytes::Bytes;
use form_urlencoded::Serializer;
use hyper::Body;
use mozjs::jsapi::Heap;
use mozjs::jsval::JSVal;

use ion::{Context, Error, ErrorKind, Result, Value};
use ion::conversions::FromValue;

use crate::globals::file::{Blob, BufferSource};
use crate::globals::url::URLSearchParams;

#[derive(Debug, Clone, Traceable)]
#[non_exhaustive]
enum FetchBodyInner {
	None,
	Bytes(#[trace(no_trace)] Bytes),
}

#[derive(Clone, Debug, Traceable)]
#[non_exhaustive]
pub enum FetchBodyKind {
	String,
	Blob(String),
	URLSearchParams,
}

impl Display for FetchBodyKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			FetchBodyKind::String => f.write_str("text/plain;charset=UTF-8"),
			FetchBodyKind::Blob(mime) => f.write_str(mime),
			FetchBodyKind::URLSearchParams => f.write_str("application/x-www-form-urlencoded;charset=UTF-8"),
		}
	}
}

#[derive(Debug, Traceable)]
pub struct FetchBody {
	body: FetchBodyInner,
	source: Option<Box<Heap<JSVal>>>,
	pub(crate) kind: Option<FetchBodyKind>,
}

impl FetchBody {
	pub fn is_none(&self) -> bool {
		matches!(&self.body, FetchBodyInner::None)
	}

	pub fn is_empty(&self) -> bool {
		match &self.body {
			FetchBodyInner::None => true,
			FetchBodyInner::Bytes(bytes) => bytes.is_empty(),
		}
	}

	pub fn len(&self) -> Option<usize> {
		match &self.body {
			FetchBodyInner::None => None,
			FetchBodyInner::Bytes(bytes) => Some(bytes.len()),
		}
	}

	pub fn is_not_stream(&self) -> bool {
		matches!(&self.body, FetchBodyInner::None | FetchBodyInner::Bytes(_))
	}

	pub fn to_http_body(&self) -> Body {
		match &self.body {
			FetchBodyInner::None => Body::empty(),
			FetchBodyInner::Bytes(bytes) => Body::from(bytes.clone()),
		}
	}
}

impl Clone for FetchBody {
	fn clone(&self) -> FetchBody {
		FetchBody {
			body: self.body.clone(),
			source: self.source.as_ref().map(|s| Heap::boxed(s.get())),
			kind: self.kind.clone(),
		}
	}
}

impl Default for FetchBody {
	fn default() -> FetchBody {
		FetchBody {
			body: FetchBodyInner::None,
			source: None,
			kind: None,
		}
	}
}

impl<'cx> FromValue<'cx> for FetchBody {
	type Config = ();
	fn from_value(cx: &'cx Context, value: &Value, strict: bool, _: ()) -> Result<FetchBody> {
		if value.handle().is_string() {
			return Ok(FetchBody {
				body: FetchBodyInner::Bytes(Bytes::from(String::from_value(cx, value, strict, ()).unwrap())),
				source: Some(Heap::boxed(value.get())),
				kind: Some(FetchBodyKind::String),
			});
		} else if value.handle().is_object() {
			if let Ok(source) = BufferSource::from_value(cx, value, strict, false) {
				return Ok(FetchBody {
					body: FetchBodyInner::Bytes(source.to_bytes()),
					source: Some(Heap::boxed(value.get())),
					kind: None,
				});
			} else if let Ok(blob) = <&Blob>::from_value(cx, value, strict, ()) {
				return Ok(FetchBody {
					body: FetchBodyInner::Bytes(blob.as_bytes().clone()),
					source: Some(Heap::boxed(value.get())),
					kind: blob.kind().map(FetchBodyKind::Blob),
				});
			} else if let Ok(search_params) = <&URLSearchParams>::from_value(cx, value, strict, ()) {
				return Ok(FetchBody {
					body: FetchBodyInner::Bytes(Bytes::from(
						Serializer::new(String::new()).extend_pairs(search_params.pairs()).finish(),
					)),
					source: Some(Heap::boxed(value.get())),
					kind: Some(FetchBodyKind::URLSearchParams),
				});
			}
		}
		Err(Error::new("Expected Valid Body", ErrorKind::Type))
	}
}
