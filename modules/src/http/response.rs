/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub use class::*;

#[js_class]
pub mod class {
	use bytes::{Buf, BufMut};
	use hyper::Body;
	use hyper::body::HttpBody;
	use ion::{Error, Result};
	use ion::typedarray::ArrayBuffer;

	use crate::http::header::Headers;

	pub struct Response {
		pub(crate) res: hyper::Response<Body>,
		pub(crate) body_used: bool,
		pub(crate) redirected: bool,
		pub(crate) url: String,
	}

	impl Response {
		#[ion(constructor)]
		pub fn constructor() -> Result<Response> {
			Err(Error::new("Constructor should not be called.", None))
		}

		pub(crate) fn new(res: hyper::Response<Body>, url: &str) -> Result<Response> {
			Ok(Response {
				res,
				body_used: false,
				redirected: false,
				url: String::from(url),
			})
		}

		#[ion(get)]
		pub fn get_bodyUsed(&self) -> bool {
			self.body_used
		}

		#[ion(get)]
		pub fn get_headers(&self) -> Headers {
			Headers::new(self.res.headers().clone())
		}

		#[ion(get)]
		pub fn get_ok(&self) -> bool {
			self.res.status().is_success()
		}

		#[ion(get)]
		pub fn get_status(&self) -> u16 {
			self.res.status().as_u16()
		}

		#[ion(get)]
		pub fn get_statusText(&self) -> Option<String> {
			self.res.status().canonical_reason().map(String::from)
		}

		async fn read_to_bytes(&mut self) -> Result<Vec<u8>> {
			if self.body_used {
				return Err(Error::new("Response body has already been used.", None));
			}
			self.body_used = true;

			let body = self.res.body_mut();

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
			Ok(ArrayBuffer { buf: bytes })
		}

		pub async fn text(&mut self) -> Result<String> {
			let bytes = self.read_to_bytes().await?;
			String::from_utf8(bytes).map_err(|e| Error::new(&format!("Invalid UTF-8 sequence: {}", e), None))
		}
	}
}
