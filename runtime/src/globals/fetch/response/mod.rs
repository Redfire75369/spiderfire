/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub use class::*;

mod options;

#[js_class]
#[ion(runtime = crate)]
pub mod class {
	use bytes::{Buf, BufMut};
	use hyper::{Body, StatusCode};
	use hyper::body::HttpBody;
	use mozjs::jsapi::{Heap, JSObject};
	use url::Url;

	use ion::{ClassDefinition, Context, Error, ErrorKind, Result};
	use ion::typedarray::ArrayBuffer;

	use crate::globals::fetch::body::FetchBody;
	use crate::globals::fetch::header::{HeadersInner, HeadersKind};
	use crate::globals::fetch::Headers;
	use crate::globals::fetch::response::options::ResponseInit;

	#[ion(into_value)]
	pub struct Response {
		pub(crate) response: hyper::Response<Body>,
		pub(crate) headers: Box<Heap<*mut JSObject>>,
		pub(crate) body: Option<FetchBody>,
		pub(crate) body_used: bool,

		pub(crate) url: Option<Url>,
		pub(crate) redirected: bool,

		pub(crate) status: Option<StatusCode>,
		pub(crate) status_text: Option<String>,
	}

	impl Response {
		pub(crate) fn new(cx: &Context, mut response: hyper::Response<Body>, url: Url, redirected: bool) -> Response {
			let headers = Headers::new_object(
				cx,
				Headers {
					headers: HeadersInner::MutRef(response.headers_mut()),
					kind: HeadersKind::Response,
				},
			);
			let status = response.status();
			let status_text = String::from(status.canonical_reason().unwrap());

			Response {
				response,
				headers: Heap::boxed(headers),
				body: None,
				body_used: false,

				url: Some(url),
				redirected,

				status: Some(status),
				status_text: Some(status_text),
			}
		}

		#[ion(constructor)]
		pub fn constructor(cx: &Context, body: Option<FetchBody>, init: Option<ResponseInit>) -> Result<Response> {
			let init = init.unwrap_or_default();

			let response = hyper::Response::builder().status(init.status).body(Body::empty())?;
			let mut response = Response {
				response,
				headers: Box::default(),
				body: None,
				body_used: false,

				url: None,
				redirected: false,

				status: Some(init.status),
				status_text: Some(init.status_text),
			};

			let headers = init
				.headers
				.into_headers(HeadersInner::MutRef(response.response.headers_mut()), HeadersKind::Response)?;
			response.headers.set(Headers::new_object(cx, headers));

			if let Some(body) = body {
				if init.status == StatusCode::NO_CONTENT || init.status == StatusCode::RESET_CONTENT || init.status == StatusCode::NOT_MODIFIED {
					return Err(Error::new("Received non-null body with null body status.", ErrorKind::Type));
				}
				// TODO: Add Content-Type Header
				response.body = Some(body);
			}

			Ok(response)
		}

		#[ion(get)]
		pub fn get_headers(&self) -> *mut JSObject {
			self.headers.get()
		}

		#[ion(get)]
		pub fn get_ok(&self) -> bool {
			self.response.status().is_success()
		}

		#[ion(get)]
		pub fn get_status(&self) -> u16 {
			self.status.as_ref().map(StatusCode::as_u16).unwrap_or(0)
		}

		#[ion(get)]
		pub fn get_status_text(&self) -> Option<String> {
			self.status_text
				.as_deref()
				.or_else(|| self.response.status().canonical_reason())
				.map(String::from)
		}

		#[ion(get)]
		pub fn get_redirected(&self) -> bool {
			self.redirected
		}

		#[ion(get)]
		pub fn get_url(&self) -> String {
			self.url.as_ref().map(Url::to_string).unwrap_or_default()
		}

		#[ion(get)]
		pub fn get_body_used(&self) -> bool {
			self.body_used
		}

		async fn read_to_bytes(&mut self) -> Result<Vec<u8>> {
			if self.body_used {
				return Err(Error::new("Response body has already been used.", None));
			}
			self.body_used = true;

			let body = self.response.body_mut();

			let first = if let Some(buf) = body.data().await {
				buf?
			} else {
				return Ok(Vec::new());
			};

			let second = if let Some(buf) = body.data().await {
				buf?
			} else {
				return Ok(first.to_vec());
			};

			let cap = first.remaining() + second.remaining() + body.size_hint().lower() as usize;
			let mut vec = Vec::with_capacity(cap);
			vec.put(first);
			vec.put(second);

			while let Some(buf) = body.data().await {
				vec.put(buf?);
			}

			Ok(vec)
		}

		pub async fn arrayBuffer(&mut self) -> Result<ArrayBuffer> {
			let bytes = self.read_to_bytes().await?;
			Ok(ArrayBuffer::from(bytes))
		}

		pub async fn text(&mut self) -> Result<String> {
			let bytes = self.read_to_bytes().await?;
			String::from_utf8(bytes).map_err(|e| Error::new(&format!("Invalid UTF-8 sequence: {}", e), None))
		}
	}
}
