/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method};
use http::header::HeaderName;
use hyper::Body;
use mozjs::conversions::{ConversionResult, FromJSValConvertible, jsstr_to_string};
use mozjs::error::throw_type_error;
use mozjs::jsapi::{ESClass, GetBuiltinClass, JSContext, Unbox};
use mozjs::jsval::JSVal;
use mozjs::rust::HandleValue;
use url::Url;

pub use class::*;
use ion::{Context, Error, Object, Result};
use runtime::globals::abort::AbortSignal;

use crate::http::client::ClientRequestOptions;
use crate::http::header::HeadersInit;

#[derive(Copy, Clone, Debug)]
pub enum Redirection {
	Follow,
	Error,
	Manual,
}

impl Default for Redirection {
	fn default() -> Redirection {
		Redirection::Follow
	}
}

impl FromJSValConvertible for Redirection {
	type Config = ();

	unsafe fn from_jsval(cx: *mut JSContext, val: HandleValue, _: ()) -> std::result::Result<ConversionResult<Redirection>, ()> {
		if val.is_string() {
			let string = jsstr_to_string(cx, val.to_string());
			match &*string.to_ascii_lowercase() {
				"follow" => Ok(ConversionResult::Success(Redirection::Follow)),
				"error" => Ok(ConversionResult::Success(Redirection::Error)),
				"manual" => Ok(ConversionResult::Success(Redirection::Manual)),
				_ => Ok(ConversionResult::Failure(Cow::Borrowed("Invalid Redirection String"))),
			}
		} else {
			throw_type_error(cx, "Redirection must be a string");
			Err(())
		}
	}
}

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
pub struct RequestOptions {
	pub(crate) auth: Option<String>,
	#[derivative(Default(value = "true"))]
	#[ion(default = true)]
	pub(crate) set_host: bool,

	#[ion(default)]
	pub(crate) client: ClientRequestOptions,
	#[ion(default)]
	pub(crate) redirect: Redirection,
	#[ion(default)]
	pub(crate) signal: AbortSignal,

	#[ion(default)]
	pub(crate) headers: HeadersInit,
	#[ion(parser = | b | parse_body(cx, b))]
	pub(crate) body: Option<Bytes>,
}

#[derive(Derivative, FromJSVal)]
#[derivative(Default)]
pub struct RequestBuilderOptions {
	pub(crate) method: Option<String>,
	#[ion(default, inherit)]
	pub(crate) options: RequestOptions,
}

impl RequestBuilderOptions {
	pub fn from_request_options<O: Into<Option<RequestOptions>>, M: Into<Option<String>>>(options: O, method: M) -> RequestBuilderOptions {
		let options = options.into().unwrap_or_default();
		let method = method.into();
		RequestBuilderOptions { method, options }
	}
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
	use runtime::globals::abort::AbortSignal;

	use crate::http::{Headers, Resource};
	use crate::http::client::ClientRequestOptions;
	use crate::http::request::{
		add_authorisation_header, add_host_header, check_method_with_body, check_url_scheme, clone_request, Redirection, RequestBuilderOptions,
	};

	#[ion(into_jsval)]
	pub struct Request {
		pub(crate) request: hyper::Request<Body>,
		pub(crate) body: Bytes,

		pub(crate) client: ClientRequestOptions,
		pub(crate) redirection: Redirection,
		pub(crate) signal: AbortSignal,
		pub(crate) url: Url,
	}

	impl Request {
		#[ion(skip)]
		pub fn clone(&self) -> Result<Request> {
			let request = clone_request(&self.request)?;
			let body = self.body.clone();

			let client = self.client.clone();
			let redirection = self.redirection;
			let signal = self.signal.clone();
			let url = self.url.clone();

			Ok(Request {
				request,
				body,
				client,
				redirection,
				signal,
				url,
			})
		}

		#[ion(constructor)]
		pub fn constructor(resource: Resource, options: Option<RequestBuilderOptions>) -> Result<Request> {
			let mut request = match resource {
				Resource::Request(request) => request.clone()?,
				Resource::String(url) => {
					let uri = Uri::from_str(&url)?;
					let url = Url::from_str(&url)?;
					let request = hyper::Request::builder().uri(uri).body(Body::empty())?;

					Request {
						request,
						body: Bytes::new(),

						client: ClientRequestOptions::default(),
						redirection: Redirection::Follow,
						signal: AbortSignal::default(),
						url,
					}
				}
			};

			check_url_scheme(&request.url)?;

			let RequestBuilderOptions { method, options } = options.unwrap_or_default();
			if let Some(mut method) = method {
				method.make_ascii_uppercase();
				let method = Method::from_str(&method)?;
				check_method_with_body(&method, options.body.is_some())?;
				*request.request.method_mut() = method;
			}

			*request.request.headers_mut() = options.headers.into_headers()?.inner();

			add_authorisation_header(request.request.headers_mut(), &request.url, options.auth)?;
			add_host_header(request.request.headers_mut(), &request.url, options.set_host)?;

			if let Some(body) = options.body {
				request.body = body;
				*request.request.body_mut() = Body::empty();
			}
			request.client = options.client;
			request.redirection = options.redirect;
			request.signal = options.signal;

			Ok(request)
		}

		#[ion(get)]
		pub fn get_headers(&self) -> Headers {
			Headers::new(self.request.headers().clone(), true)
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

macro_rules! typedarray_to_bytes {
	($body:expr) => {
		Bytes::default()
	};
	($body:expr, [$arr:ident, true]$(, $($rest:tt)*)?) => {
		paste::paste! {
			if let Ok(arr) = <::mozjs::typedarray::$arr>::from($body) {
				Bytes::copy_from_slice(arr.as_slice())
			} else if let Ok(arr) = <::mozjs::typedarray::[<Heap $arr>]>::from($body) {
				Bytes::copy_from_slice(arr.as_slice())
			} else {
				typedarray_to_bytes!($body$(, $($rest)*)?)
			}
		}
	};
	($body:expr, [$arr:ident, false]$(, $($rest:tt)*)?) => {
		paste::paste! {
			if let Ok(arr) = <::mozjs::typedarray::$arr>::from($body) {
				let bytes: &[u8] = cast_slice(arr.as_slice());
				Bytes::copy_from_slice(bytes)
			} else if let Ok(arr) = <::mozjs::typedarray::[<Heap $arr>]>::from($body) {
				let bytes: &[u8] = cast_slice(arr.as_slice());
				Bytes::copy_from_slice(bytes)
			} else {
				typedarray_to_bytes!($body$(, $($rest)*)?)
			}
		}
	};
}

pub(crate) unsafe fn parse_body(cx: Context, body: JSVal) -> Bytes {
	if body.is_string() {
		Bytes::from(jsstr_to_string(cx, body.to_string()))
	} else if body.is_object() {
		let body = body.to_object();
		rooted!(in(cx) let rbody = body);

		let mut class = ESClass::Other;
		if GetBuiltinClass(cx, rbody.handle().into(), &mut class) {
			if class == ESClass::String {
				rooted!(in(cx) let mut unboxed = Object::new(cx).to_value());

				if Unbox(cx, rbody.handle().into(), unboxed.handle_mut().into()) {
					return Bytes::from(jsstr_to_string(cx, unboxed.get().to_string()));
				}
			}
		} else {
			return Bytes::default();
		}

		typedarray_to_bytes!(body, [ArrayBuffer, true], [ArrayBufferView, true])
	} else {
		Bytes::default()
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
	if url.scheme() == "http" {
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
