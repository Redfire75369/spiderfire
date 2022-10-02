/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use bytes::Bytes;

pub use class::*;

use crate::http::header::Headers;
use crate::http::options::parse_body;

#[derive(FromJSVal)]
pub enum Resource {
	#[ion(inherit)]
	Request(Request),
	#[ion(inherit)]
	String(String),
}

#[derive(Derivative, FromJSVal)]
#[derivative(Default)]
pub struct RequestBuilderOptions {
	pub(crate) method: Option<String>,
	#[derivative(Default(value = "true"))]
	#[ion(default = true)]
	pub(crate) set_host: bool,
	#[ion(inherit)]
	pub(crate) headers: Headers,
	#[ion(default, parser = |b| parse_body(cx, b))]
	pub(crate) body: Bytes,
}

#[js_class]
pub mod class {
	use bytes::Bytes;
	use http::request::Builder;
	use hyper::{Method, Uri};
	use ion::{Error, Result, ClassInitialiser, Context, Object};
	use std::str::FromStr;
	use std::borrow::Cow;
	use std::result;
	use mozjs::conversions::{ConversionResult, FromJSValConvertible};
	use mozjs::rust::HandleValue;
	use ion::error::ThrowException;

	use crate::http::request::{RequestBuilderOptions, Resource};

	pub struct Request {
		pub(crate) req: Builder,
		pub(crate) set_host: bool,
		pub(crate) body: Bytes,
	}

	impl Request {
		#[ion(constructor)]
		pub fn constructor(resource: Resource, options: Option<RequestBuilderOptions>) -> Result<Request> {
			let mut request = match resource {
				Resource::Request(request) => request.clone()?,
				Resource::String(url) => {
					let uri: Uri = url.parse()?;
					let req = hyper::Request::builder().uri(uri);

					Request { req, set_host: true, body: Bytes::new() }
				}
			};
			let options = options.unwrap_or_default();
			if let Some(mut method) = options.method {
				method.make_ascii_uppercase();
				let method = Method::from_str(&method)?;
				request.req = request.req.method(method);
			}

			if let Some(h) = request.req.headers_mut() {
				*h = options.headers.inner();
			}

			request.set_host = options.set_host;
			request.body = options.body;

			Ok(request)
		}

		#[ion(internal)]
		pub fn clone(&self) -> Result<Request> {
			let error: Result<Request> = Err(Error::new("Error in Request", None));

			let method = if let Some(method) = self.req.method_ref() {
				method.clone()
			} else {
				return error;
			};
			let uri = if let Some(uri) = self.req.uri_ref() {
				uri.clone()
			} else {
				return error;
			};
			let headers = if let Some(headers) = self.req.headers_ref() {
				headers.clone()
			} else {
				return error;
			};
			let set_host = self.set_host;
			let body = self.body.clone();

			let mut req = hyper::Request::builder().method(method).uri(uri);
			if let Some(h) = req.headers_mut() {
				*h = headers;
			}

			Ok(Request { req, set_host, body })
		}
	}

	impl FromJSValConvertible for Request {
		type Config = ();

		unsafe fn from_jsval(cx: Context, val: HandleValue, _: ()) -> result::Result<ConversionResult<Self>, ()> {
			match Object::from_jsval(cx, val, ())? {
				ConversionResult::Success(obj) => {
					if Request::instance_of(cx, obj, None) {
						match Request::get_private(cx, obj, None) {
							Ok(request) => match request.clone() {
								Ok(request) => Ok(ConversionResult::Success(request)),
								Err(err) => {
									err.throw(cx);
									Err(())
								}
							},
							Err(err) => {
								err.throw(cx);
								Err(())
							}
						}
					} else {
						Ok(ConversionResult::Failure(Cow::Borrowed("Object is not a Request")))
					}
				}
				ConversionResult::Failure(e) => Ok(ConversionResult::Failure(e)),
			}
		}
	}
}
