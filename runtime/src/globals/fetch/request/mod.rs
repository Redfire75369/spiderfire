/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::str::FromStr;

use http::{HeaderMap, HeaderValue};
use http::header::CONTENT_TYPE;
use hyper::{Body, Method, Uri};
use mozjs::jsapi::{Heap, JSObject};
use url::Url;

use ion::{ClassDefinition, Context, Error, ErrorKind, Result};
use ion::class::Reflector;
pub use options::*;

use crate::globals::abort::AbortSignal;
use crate::globals::fetch::body::FetchBody;
use crate::globals::fetch::header::HeadersKind;
use crate::globals::fetch::Headers;

mod options;

#[derive(FromValue)]
pub enum RequestInfo<'cx> {
	#[ion(inherit)]
	Request(&'cx Request),
	#[ion(inherit)]
	String(String),
}

#[js_class]
pub struct Request {
	reflector: Reflector,

	#[ion(no_trace)]
	pub(crate) request: hyper::Request<Body>,
	pub(crate) headers: Box<Heap<*mut JSObject>>,
	pub(crate) body: FetchBody,
	pub(crate) body_used: bool,

	#[ion(no_trace)]
	pub(crate) url: Url,
	#[ion(no_trace)]
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
	pub(crate) keepalive: bool,

	pub(crate) client_window: bool,
	pub(crate) signal_object: Box<Heap<*mut JSObject>>,
}

#[js_class]
impl Request {
	#[ion(constructor)]
	pub fn constructor(cx: &Context, info: RequestInfo, init: Option<RequestInit>) -> Result<Request> {
		let mut fallback_cors = false;

		let mut request = match info {
			RequestInfo::Request(request) => request.clone(),
			RequestInfo::String(url) => {
				let uri = Uri::from_str(&url)?;
				let url = Url::from_str(&url)?;
				if url.username() != "" || url.password().is_some() {
					return Err(Error::new("Received URL with embedded credentials", ErrorKind::Type));
				}
				let request = hyper::Request::builder().uri(uri).body(Body::empty())?;

				fallback_cors = true;

				Request {
					reflector: Reflector::default(),

					request,
					headers: Box::default(),
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

					client_window: true,
					signal_object: Heap::boxed(AbortSignal::new_object(cx, Box::default())),
				}
			}
		};

		let mut headers = None;
		let mut body = None;

		if let Some(init) = init {
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
				request.signal_object.set(signal_object);
			}

			if let Some(mut method) = init.method {
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
			if method != Method::GET && method != Method::HEAD && method != Method::POST {
				return Err(Error::new("Invalid request method", ErrorKind::Type));
			}
		}

		let kind = if request.mode == RequestMode::NoCors {
			HeadersKind::RequestNoCors
		} else {
			HeadersKind::Request
		};

		let mut headers = if let Some(headers) = headers {
			headers.into_headers(HeaderMap::new(), kind)?
		} else {
			Headers {
				reflector: Reflector::default(),
				headers: HeaderMap::new(),
				kind,
			}
		};

		if let Some(body) = body {
			if let Some(kind) = &body.kind {
				if !headers.headers.contains_key(CONTENT_TYPE) {
					headers.headers.append(CONTENT_TYPE, HeaderValue::from_str(&kind.to_string()).unwrap());
				}
			}

			request.body = body;
		}
		request.headers.set(Headers::new_object(cx, Box::new(headers)));

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
	pub fn get_headers(&self) -> *mut JSObject {
		self.headers.get()
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
	pub fn get_keepalive(&self) -> bool {
		self.keepalive
	}

	#[ion(get)]
	pub fn get_is_reload_navigation(&self) -> bool {
		false
	}

	#[ion(get)]
	pub fn get_is_history_navigation(&self) -> bool {
		false
	}

	#[ion(get)]
	pub fn get_signal(&self) -> *mut JSObject {
		self.signal_object.get()
	}

	#[ion(get)]
	pub fn get_duplex(&self) -> String {
		String::from("half")
	}
}

impl Clone for Request {
	fn clone(&self) -> Request {
		let method = self.request.method().clone();
		let uri = self.request.uri().clone();

		let request = hyper::Request::builder().method(method).uri(uri);
		let request = request.body(Body::empty()).unwrap();
		let url = self.locations.last().unwrap().clone();

		Request {
			reflector: Reflector::default(),

			request,
			headers: Box::default(),
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

			client_window: self.client_window,
			signal_object: Heap::boxed(self.signal_object.get()),
		}
	}
}
