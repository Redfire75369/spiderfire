/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use hyper::Body;

pub use class::*;
use ion::Result;
pub use options::*;

mod options;

#[allow(clippy::large_enum_variant)]
#[derive(FromValue)]
pub enum RequestInfo {
	#[ion(inherit)]
	Request(Request),
	#[ion(inherit)]
	String(String),
}

#[js_class]
pub mod class {
	use std::str::FromStr;
	use http::header::CONTENT_TYPE;
	use http::HeaderValue;

	use hyper::{Body, Method, Uri};
	use url::Url;

	use mozjs::jsapi::{Heap, JSObject, JSTracer};
	use mozjs::gc::Traceable;
	use ion::{ClassDefinition, Context, Error, ErrorKind, Object, Result, Value};
	use ion::conversions::{FromValue, ToValue};

	use crate::globals::abort::AbortSignal;
	use crate::globals::fetch::{Headers, RequestInfo};
	use crate::globals::fetch::body::FetchBody;
	use crate::globals::fetch::header::HeadersKind;
	use crate::globals::fetch::request::{
		clone_request, Referrer, ReferrerPolicy, RequestBuilderInit, RequestCache, RequestCredentials, RequestMode, RequestRedirect,
	};

	#[ion(into_value)]
	pub struct Request {
		pub(crate) request: hyper::Request<Body>,
		pub(crate) body: FetchBody,
		pub(crate) body_used: bool,

		pub(crate) url: Url,
		pub(crate) locations: Vec<Url>,

		pub(crate) referrer: Referrer,
		pub(crate) referrer_policy: ReferrerPolicy,

		pub(crate) mode: RequestMode,
		pub(crate) credentials: RequestCredentials,
		pub(crate) cache: RequestCache,
		pub(crate) redirect: RequestRedirect,

		pub(crate) integrity: String,

		#[allow(dead_code)]
		pub(crate) unsafe_request: bool,
		#[ion(readonly)]
		pub keepalive: bool,
		#[ion(readonly, name = "isReloadNavigation")]
		pub reload_navigation: bool,
		#[ion(readonly, name = "isHistoryNavigation")]
		pub history_navigation: bool,

		pub(crate) client_window: bool,
		pub(crate) signal: AbortSignal,
		pub(crate) signal_object: Box<Heap<*mut JSObject>>,
	}

	impl Request {
		#[ion(constructor)]
		pub fn constructor(cx: &Context, info: RequestInfo, init: Option<RequestBuilderInit>) -> Result<Request> {
			let mut fallback_cors = false;

			let mut request = match info {
				RequestInfo::Request(request) => request.clone()?,
				RequestInfo::String(url) => {
					let uri = Uri::from_str(&url)?;
					let url = Url::from_str(&url)?;
					if url.username() != "" || url.password().is_some() {
						return Err(Error::new("Received URL with embedded credentials", ErrorKind::Type));
					}
					let request = hyper::Request::builder().uri(uri).body(Body::empty())?;

					fallback_cors = true;

					let signal = AbortSignal::default();
					let signal_object = Heap::boxed(AbortSignal::new_object(cx, signal.clone()));

					Request {
						request,
						body: FetchBody::default(),
						body_used: false,

						url: url.clone(),
						locations: vec![url],

						referrer: Referrer::default(),
						referrer_policy: ReferrerPolicy::default(),

						mode: RequestMode::default(),
						credentials: RequestCredentials::default(),
						cache: RequestCache::default(),
						redirect: RequestRedirect::default(),

						integrity: String::new(),

						unsafe_request: false,
						keepalive: false,
						reload_navigation: false,
						history_navigation: false,

						client_window: true,
						signal,
						signal_object,
					}
				}
			};

			let mut headers = None;
			let mut body = None;

			if let Some(RequestBuilderInit { method, init }) = init {
				if let Some(window) = init.window {
					if window.is_null() {
						request.client_window = false;
					} else {
						return Err(Error::new("Received non-null window type", ErrorKind::Type));
					}
				}

				if request.mode == RequestMode::Navigate {
					request.mode = RequestMode::SameOrigin;
				}
				request.reload_navigation = false;
				request.history_navigation = false;

				if let Some(referrer) = init.referrer {
					request.referrer = referrer;
				}
				if let Some(policy) = init.referrer_policy {
					request.referrer_policy = policy;
				}

				let mode = init.mode.or(fallback_cors.then_some(RequestMode::Cors));
				if let Some(mode) = mode {
					if mode == RequestMode::Navigate {
						return Err(Error::new("Received 'navigate' mode", ErrorKind::Type));
					}
					request.mode = mode;
				}

				if let Some(credentials) = init.credentials {
					request.credentials = credentials;
				}
				if let Some(cache) = init.cache {
					request.cache = cache;
				}
				if let Some(redirect) = init.redirect {
					request.redirect = redirect;
				}
				if let Some(integrity) = init.integrity {
					request.integrity = integrity;
				}
				if let Some(keepalive) = init.keepalive {
					request.keepalive = keepalive;
				}

				if let Some(signal_object) = init.signal {
					let signal = Object::from(cx.root_object(signal_object)).as_value(cx);
					request.signal = AbortSignal::from_value(cx, &signal, false, ())?;
					request.signal_object = Heap::boxed(signal_object);
				}

				if let Some(mut method) = method {
					method.make_ascii_uppercase();
					let method = Method::from_str(&method)?;
					if method == Method::CONNECT || method == Method::TRACE {
						return Err(Error::new("Received invalid request method", ErrorKind::Type));
					}
					*request.request.method_mut() = method;
				}

				headers = init.headers;
				body = init.body;
			}

			if request.cache == RequestCache::OnlyIfCached && request.mode != RequestMode::SameOrigin {
				return Err(Error::new(
					"Request cache mode 'only-if-cached' can only be used with request mode 'same-origin'",
					ErrorKind::Type,
				));
			}

			if request.mode == RequestMode::NoCors {
				let method = request.request.method();
				if method != Method::GET || method != Method::HEAD || method != Method::POST {
					return Err(Error::new("Invalid request method.", ErrorKind::Type));
				}
			}

			if let Some(headers) = headers {
				*request.request.headers_mut() = headers.into_headers(HeadersKind::Request)?.headers;
			}

			if let Some(body) = body {
				if let Some(kind) = &body.kind {
					let headers = request.request.headers_mut();
					if headers.contains_key(CONTENT_TYPE) {
						headers.append(CONTENT_TYPE, HeaderValue::from_str(&kind.to_string())?);
					}
				}

				request.body = body;
			}

			Ok(request)
		}

		#[ion(get)]
		pub fn get_method(&self) -> String {
			self.request.method().to_string()
		}

		#[ion(get)]
		pub fn get_url(&self) -> String {
			self.request.uri().to_string()
		}

		#[ion(get)]
		pub fn get_destination(&self) -> String {
			String::new()
		}

		#[ion(get)]
		pub fn get_referrer(&self) -> String {
			self.referrer.to_string()
		}

		#[ion(get)]
		pub fn get_referrer_policy(&self) -> String {
			self.referrer.to_string()
		}

		#[ion(get)]
		pub fn get_mode(&self) -> String {
			self.mode.to_string()
		}

		#[ion(get)]
		pub fn get_credentials(&self) -> String {
			self.credentials.to_string()
		}

		#[ion(get)]
		pub fn get_cache(&self) -> String {
			self.cache.to_string()
		}

		#[ion(get)]
		pub fn get_redirect(&self) -> String {
			self.redirect.to_string()
		}

		#[ion(get)]
		pub fn get_integrity(&self) -> String {
			self.integrity.clone()
		}

		#[ion(get)]
		pub fn get_signal(&self) -> *mut JSObject {
			self.signal_object.get()
		}

		#[ion(get)]
		pub fn get_duplex(&self) -> String {
			String::from("half")
		}

		#[allow(clippy::should_implement_trait)]
		#[ion(skip)]
		pub fn clone(&self) -> Result<Request> {
			let request = clone_request(&self.request)?;
			let url = self.locations.last().unwrap().clone();

			Ok(Request {
				request,
				body: self.body.clone(),
				body_used: self.body_used,

				url: url.clone(),
				locations: vec![url],

				referrer: self.referrer.clone(),
				referrer_policy: self.referrer_policy,

				mode: self.mode,
				credentials: self.credentials,
				cache: self.cache,
				redirect: self.redirect,

				integrity: self.integrity.clone(),

				unsafe_request: true,
				keepalive: self.keepalive,
				reload_navigation: false,
				history_navigation: false,

				client_window: self.client_window,
				signal: self.signal.clone(),
				signal_object: Heap::boxed(self.signal_object.get()),
			})
		}

		#[ion(get)]
		pub fn get_headers(&self) -> Headers {
			Headers {
				headers: self.request.headers().clone(),
				kind: HeadersKind::Request,
			}
		}
	}

	unsafe impl Traceable for Request {
		unsafe fn trace(&self, trc: *mut JSTracer) {
			unsafe {
				self.signal_object.trace(trc);
			}
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
