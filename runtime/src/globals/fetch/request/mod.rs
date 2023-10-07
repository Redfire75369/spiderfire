/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use http::{HeaderMap, HeaderValue, Method};
use http::header::HeaderName;
use hyper::Body;
use url::Url;

pub use class::*;
use ion::{Error, Result};
pub use options::*;

mod options;

#[allow(clippy::large_enum_variant)]
#[derive(FromValue)]
pub enum Resource {
	#[ion(inherit)]
	Request(Request),
	#[ion(inherit)]
	String(String),
}

#[js_class]
pub mod class {
	use std::str::FromStr;

	use hyper::{Body, Method, Uri};
	use url::Url;

	use ion::{ClassDefinition, Context, Error, ErrorKind, Object, Result, Value};
	use ion::conversions::FromValue;

	use crate::globals::abort::AbortSignal;
	use crate::globals::fetch::{Headers, Resource};
	use crate::globals::fetch::body::FetchBody;
	use crate::globals::fetch::request::{
		add_authorisation_header, add_host_header, check_method_with_body, check_url_scheme, clone_request, RequestBuilderInit, RequestRedirect,
	};

	#[ion(into_value)]
	pub struct Request {
		pub(crate) request: hyper::Request<Body>,
		pub(crate) body: FetchBody,

		pub(crate) redirect: RequestRedirect,
		pub(crate) signal: AbortSignal,
		pub(crate) url: Url,
	}

	impl Request {
		#[allow(clippy::should_implement_trait)]
		#[ion(skip)]
		pub fn clone(&self) -> Result<Request> {
			let request = clone_request(&self.request)?;
			let body = self.body.clone();

			let redirect = self.redirect;
			let signal = self.signal.clone();
			let url = self.url.clone();

			Ok(Request { request, body, redirect, signal, url })
		}

		#[ion(constructor)]
		pub fn constructor(resource: Resource, init: Option<RequestBuilderInit>) -> Result<Request> {
			let mut request = match resource {
				Resource::Request(request) => request.clone()?,
				Resource::String(url) => {
					let uri = Uri::from_str(&url)?;
					let url = Url::from_str(&url)?;
					let request = hyper::Request::builder().uri(uri).body(Body::empty())?;

					Request {
						request,
						body: FetchBody::default(),

						redirect: RequestRedirect::Follow,
						signal: AbortSignal::default(),
						url,
					}
				}
			};

			check_url_scheme(&request.url)?;

			let RequestBuilderInit { method, init } = init.unwrap_or_default();
			if let Some(mut method) = method {
				method.make_ascii_uppercase();
				let method = Method::from_str(&method)?;
				check_method_with_body(&method, init.body.is_some())?;
				*request.request.method_mut() = method;
			}

			*request.request.headers_mut() = init.headers.into_headers()?.headers;

			add_authorisation_header(request.request.headers_mut(), &request.url, init.auth)?;
			add_host_header(request.request.headers_mut(), &request.url, init.set_host)?;

			if let Some(body) = init.body {
				request.body = body;
			}
			request.redirect = init.redirect;
			request.signal = init.signal;

			Ok(request)
		}

		#[ion(get)]
		pub fn get_headers(&self) -> Headers {
			Headers::new(self.request.headers().clone(), true)
		}
	}

	impl<'cx> FromValue<'cx> for Request {
		type Config = ();
		fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<Request>
		where
			'cx: 'v,
		{
			let object = Object::from_value(cx, value, true, ())?;
			if Request::instance_of(cx, &object, None) {
				Request::get_private(&object).clone()
			} else {
				Err(Error::new("Expected Request", ErrorKind::Type))
			}
		}
	}
}

pub(crate) fn clone_request(request: &hyper::Request<Body>) -> Result<hyper::Request<Body>> {
	let method = request.method().clone();
	let uri = request.uri().clone();
	let headers = request.headers().clone();

	let mut request = hyper::Request::builder().method(method).uri(uri);
	if let Some(head) = request.headers_mut() {
		*head = headers;
	}

	let request = request.body(Body::empty())?;
	Ok(request)
}

pub(crate) fn check_url_scheme(url: &Url) -> Result<()> {
	if url.scheme() == "https" || url.scheme() == "http" {
		Ok(())
	} else {
		Err(Error::new("Invalid Scheme", None))
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
