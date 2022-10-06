/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use http::{HeaderMap, HeaderValue, Method};
use http::header::HeaderName;
use url::Url;

pub use class::*;
use ion::{Error, Result};

use crate::http::options::RequestOptions;

#[allow(clippy::large_enum_variant)]
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
	#[ion(default, inherit)]
	pub(crate) options: RequestOptions,
}

#[js_class]
pub mod class {
	use std::borrow::Cow;
	use std::result;
	use std::str::FromStr;

	use bytes::Bytes;
	use hyper::{Body, Method, Uri};
	use mozjs::conversions::{ConversionResult, FromJSValConvertible};
	use mozjs::rust::HandleValue;
	use url::Url;

	use ion::{ClassInitialiser, Context, Object, Result};
	use ion::error::ThrowException;

	use crate::http::client::ClientRequestOptions;
	use crate::http::request::{add_authorisation_header, add_host_header, check_method_with_body, RequestBuilderOptions, Resource};

	pub struct Request {
		pub(crate) request: hyper::Request<Body>,
		pub(crate) body: Bytes,
		pub(crate) client: ClientRequestOptions,
	}

	impl Request {
		#[ion(constructor)]
		pub fn constructor(resource: Resource, options: Option<RequestBuilderOptions>) -> Result<Request> {
			let mut request = match resource {
				Resource::Request(request) => request.clone()?,
				Resource::String(url) => {
					let uri = Uri::from_str(&url)?;
					let request = hyper::Request::builder().uri(uri).body(Body::empty())?;

					Request {
						request,
						body: Bytes::new(),
						client: ClientRequestOptions::default(),
					}
				}
			};

			let url = Url::from_str(&request.request.uri().to_string())?;

			let RequestBuilderOptions { method, options } = options.unwrap_or_default();
			if let Some(mut method) = method {
				method.make_ascii_uppercase();
				let method = Method::from_str(&method)?;
				check_method_with_body(&method, options.body.is_some())?;
				*request.request.method_mut() = method;
			}

			*request.request.headers_mut() = options.headers.into_headers()?.inner();

			add_authorisation_header(request.request.headers_mut(), &url, options.auth)?;
			add_host_header(request.request.headers_mut(), &url, options.set_host)?;

			if let Some(body) = options.body {
				request.body = body;
				*request.request.body_mut() = Body::empty();
			}
			request.client = options.client;

			Ok(request)
		}

		#[ion(internal)]
		pub fn clone(&self) -> Result<Request> {
			let method = self.request.method().clone();
			let uri = self.request.uri().clone();
			let headers = self.request.headers().clone();

			let body = self.body.clone();
			let client = self.client.clone();

			let mut request = hyper::Request::builder().method(method).uri(uri);
			if let Some(head) = request.headers_mut() {
				*head = headers;
			}
			let request = request.body(Body::empty())?;

			Ok(Request { request, body, client })
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

pub(crate) fn check_method_with_body(method: &Method, has_body: bool) -> Result<()> {
	match (has_body, method) {
		(true, &Method::GET | &Method::HEAD | &Method::CONNECT | &Method::OPTIONS | &Method::TRACE) => {
			Err(Error::new(&format!("{} cannot have a body.", method.as_str()), None))
		}
		(false, &Method::POST | &Method::PUT | &Method::PATCH) => Err(Error::new(&format!("{} must have a body.", method.as_str()), None)),
		_ => Ok(()),
	}
}

pub(crate) fn add_authorisation_header(headers: &mut HeaderMap, url: &Url, auth: Option<String>) -> Result<()> {
	let auth = url.password().map(|pw| format!("{}:{}", url.username(), pw)).or(auth);

	if let Some(auth) = auth {
		let auth = HeaderValue::from_str(&auth)?;
		if !headers.contains_key("authorization") {
			headers.insert(HeaderName::from_static("authorization"), auth);
		}
	}
	Ok(())
}

pub(crate) fn add_host_header(headers: &mut HeaderMap, url: &Url, set_host: bool) -> Result<()> {
	if set_host {
		let host = url.host_str().map(|host| {
			if let Some(port) = url.port() {
				format!("{}:{}", host, port)
			} else {
				String::from(host)
			}
		});
		if let Some(host) = host {
			let host = HeaderValue::from_str(&host)?;
			headers.append(HeaderName::from_static("host"), host);
		}
	}
	Ok(())
}
