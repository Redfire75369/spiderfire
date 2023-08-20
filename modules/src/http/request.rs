/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method};
use http::header::HeaderName;
use hyper::Body;
use mozjs::jsapi::{ESClass, GetBuiltinClass, Unbox};
use url::Url;

pub use class::*;
use ion::{Context, Error, ErrorKind, Result, Value};
use ion::conversions::FromValue;
use runtime::globals::abort::AbortSignal;

use crate::http::client::ClientRequestOptions;
use crate::http::header::HeadersInit;

#[derive(Copy, Clone, Debug, Default)]
pub enum Redirection {
	#[default]
	Follow,
	Error,
	Manual,
}

impl<'cx> FromValue<'cx> for Redirection {
	type Config = ();
	unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<Self>
	where
		'cx: 'v,
	{
		let string = String::from_value(cx, value, true, ())?;
		match &*string.to_ascii_lowercase() {
			"follow" => Ok(Redirection::Follow),
			"error" => Ok(Redirection::Error),
			"manual" => Ok(Redirection::Manual),
			_ => Err(Error::new("Invalid Redirection String", ErrorKind::Type)),
		}
	}
}

#[allow(clippy::large_enum_variant)]
#[derive(FromValue)]
pub enum Resource {
	#[ion(inherit)]
	Request(Request),
	#[ion(inherit)]
	String(String),
}

#[derive(Derivative, FromValue)]
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
	#[ion(default, parser = |b| parse_body(cx, b))]
	pub(crate) body: Option<Bytes>,
}

#[derive(Derivative, FromValue)]
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
	use std::str::FromStr;

	use bytes::Bytes;
	use hyper::{Body, Method, Uri};
	use url::Url;

	use ion::{ClassInitialiser, Context, Error, ErrorKind, Object, Result, Value};
	use ion::conversions::FromValue;
	use runtime::globals::abort::AbortSignal;

	use crate::http::{Headers, Resource};
	use crate::http::client::ClientRequestOptions;
	use crate::http::request::{
		add_authorisation_header, add_host_header, check_method_with_body, check_url_scheme, clone_request, Redirection, RequestBuilderOptions,
	};

	#[ion(into_value)]
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

	impl<'cx> FromValue<'cx> for Request {
		type Config = ();
		unsafe fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<Request>
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

macro_rules! typedarray_to_bytes {
	($body:expr) => {
		Err(Error::new("Expected TypedArray or ArrayBuffer", ErrorKind::Type))
	};
	($body:expr, [$arr:ident, true]$(, $($rest:tt)*)?) => {
		paste::paste! {
			if let Ok(arr) = <::mozjs::typedarray::$arr>::from($body) {
				Ok(Bytes::copy_from_slice(arr.as_slice()))
			} else if let Ok(arr) = <::mozjs::typedarray::[<Heap $arr>]>::from($body) {
				Ok(Bytes::copy_from_slice(arr.as_slice()))
			} else {
				typedarray_to_bytes!($body$(, $($rest)*)?)
			}
		}
	};
	($body:expr, [$arr:ident, false]$(, $($rest:tt)*)?) => {
		paste::paste! {
			if let Ok(arr) = <::mozjs::typedarray::$arr>::from($body) {
				let bytes: &[u8] = cast_slice(arr.as_slice());
				Ok(Bytes::copy_from_slice(bytes))
			} else if let Ok(arr) = <::mozjs::typedarray::[<Heap $arr>]>::from($body) {
				let bytes: &[u8] = cast_slice(arr.as_slice());
				Ok(Bytes::copy_from_slice(bytes))
			} else {
				typedarray_to_bytes!($body$(, $($rest)*)?)
			}
		}
	};
}

pub(crate) unsafe fn parse_body<'cx: 'v, 'v>(cx: &'cx Context, body: Value<'v>) -> Result<Bytes> {
	if body.handle().is_string() {
		Ok(Bytes::from(String::from_value(cx, &body, true, ()).unwrap()))
	} else if body.handle().is_object() {
		let body = body.to_object(cx);

		let mut class = ESClass::Other;
		if GetBuiltinClass(cx.as_ptr(), body.handle().into(), &mut class) {
			if class == ESClass::String {
				let mut unboxed = Value::undefined(cx);

				if Unbox(cx.as_ptr(), body.handle().into(), unboxed.handle_mut().into()) {
					return Ok(Bytes::from(String::from_value(cx, &unboxed, true, ()).unwrap()));
				}
			}
		} else {
			return Err(Error::new("Failed to Get Class of Input", ErrorKind::Type));
		}

		typedarray_to_bytes!(body.handle().get(), [ArrayBuffer, true], [ArrayBufferView, true])
	} else {
		Err(Error::new("Expected Body to be String or Object", ErrorKind::Type))
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
