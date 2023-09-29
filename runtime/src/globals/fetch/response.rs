/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub use class::*;

#[js_class]
#[ion(runtime = crate)]
pub mod class {
	use bytes::{Buf, BufMut};
	use hyper::Body;
	use hyper::body::HttpBody;
	use url::Url;

	use ion::{Error, Result};
	use ion::typedarray::ArrayBuffer;

	use crate::globals::fetch::Headers;

	#[ion(no_constructor, into_value)]
	pub struct Response {
		pub(crate) response: hyper::Response<Body>,
		pub(crate) body_used: bool,
		pub(crate) redirections: u8,
		pub(crate) locations: Vec<Url>,
	}

	impl Response {
		pub(crate) fn new(response: hyper::Response<Body>, redirections: u8, locations: Vec<Url>) -> Response {
			Response {
				response,
				body_used: false,
				redirections,
				locations,
			}
		}

		#[ion(get)]
		pub fn get_body_used(&self) -> bool {
			self.body_used
		}

		#[ion(get)]
		pub fn get_headers(&self) -> Headers {
			Headers::new(self.response.headers().clone(), true)
		}

		#[ion(get)]
		pub fn get_ok(&self) -> bool {
			self.response.status().is_success()
		}

		#[ion(get)]
		pub fn get_status(&self) -> u16 {
			self.response.status().as_u16()
		}

		#[ion(get)]
		pub fn get_status_text(&self) -> Option<String> {
			self.response.status().canonical_reason().map(String::from)
		}

		#[ion(get)]
		pub fn get_redirected(&self) -> bool {
			self.redirections >= 1
		}

		#[ion(get)]
		pub fn get_locations(&self) -> Vec<String> {
			self.locations.iter().map(|u| String::from(u.as_str())).collect()
		}

		#[ion(get)]
		pub fn get_url(&self) -> String {
			String::from(self.locations[self.locations.len() - 1].as_str())
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
