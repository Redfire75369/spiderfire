/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};

use bytes::Bytes;
use hyper::Body;
use mozjs::jsapi::Heap;
use mozjs::jsval::JSVal;

use ion::{Context, Error, ErrorKind, Result, Value};
use ion::conversions::FromValue;

use crate::globals::file::buffer_source_to_bytes;

#[derive(Debug, Clone, Traceable)]
#[non_exhaustive]
enum FetchBodyInner {
	None,
	Bytes(#[ion(no_trace)] Bytes),
}

#[derive(Copy, Clone, Debug, Traceable)]
#[non_exhaustive]
pub enum FetchBodyKind {
	String,
}

impl Display for FetchBodyKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			FetchBodyKind::String => f.write_str("text/plain;charset=UTF-8"),
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
			kind: self.kind,
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
	fn from_value(cx: &'cx Context, value: &Value, _: bool, _: Self::Config) -> Result<FetchBody> {
		if value.handle().is_string() {
			Ok(FetchBody {
				body: FetchBodyInner::Bytes(Bytes::from(String::from_value(cx, value, true, ()).unwrap())),
				source: Some(Heap::boxed(value.get())),
				kind: Some(FetchBodyKind::String),
			})
		} else if value.handle().is_object() {
			let bytes = buffer_source_to_bytes(&value.to_object(cx))?;
			Ok(FetchBody {
				body: FetchBodyInner::Bytes(bytes),
				source: Some(Heap::boxed(value.get())),
				kind: None,
			})
		} else {
			Err(Error::new("Expected Body to be String or Object", ErrorKind::Type))
		}
	}
}
