/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::str::FromStr;

use hyper::{Body, Client, Method, Request, Uri};
use hyper::header::{HeaderName, HeaderValue};
use mozjs::jsapi::JSFunctionSpec;
use url::Url;

use ion::{ClassInitialiser, Context, Error, Object, Result};
use runtime::modules::NativeModule;

use crate::http::header::insert_header;
use crate::http::options::RequestOptions;
use crate::http::request::Resource;
use crate::http::response::Response;

fn construct_request(url: &Url, method: Method, options: RequestOptions) -> Result<Request<Body>> {
	let uri: Uri = url.as_str().parse()?;

	if options.body.is_some() {
		match method {
			Method::GET | Method::HEAD | Method::CONNECT | Method::OPTIONS | Method::TRACE => {
				return Err(Error::new(&format!("{} cannot have a body.", method.as_str()), None));
			}
			_ => {}
		}
	} else {
		match method {
			Method::POST | Method::PUT | Method::PATCH => {
				return Err(Error::new(&format!("{} must have a body.", method.as_str()), None));
			}
			_ => {}
		}
	}

	let mut request = Request::builder().method(method).uri(uri);

	let auth = url.password().map(|pw| format!("{}:{}", url.username(), pw));
	let auth = auth.or_else(|| options.auth);

	if let Some(auth) = auth {
		request.headers_mut().map(|h| {
			if let Ok(auth) = HeaderValue::from_str(&auth) {
				if !h.contains_key("authorization") {
					insert_header(h, HeaderName::from_static("authorization"), auth, true);
				}
			}
		});
	}

	if options.set_host {
		let host = url.host_str().map(|host| {
			if let Some(port) = url.port() {
				format!("{}:{}", host, port)
			} else {
				String::from(host)
			}
		});
		if let Some(host) = host {
			request.headers_mut().map(|h| {
				if let Ok(host) = HeaderValue::from_str(&host) {
					if !h.contains_key("host") {
						insert_header(h, HeaderName::from_static("host"), host, true);
					}
				}
			});
		}
	}

	if let Some(headers) = request.headers_mut() {
		*headers = options.headers.inner();
	}

	Ok(request.body(Body::from(options.body.unwrap_or_default()))?)
}

async fn request_internal(request: Request<Body>, set_host: bool) -> Result<hyper::Response<Body>> {
	let mut builder = Client::builder();
	builder.set_host(set_host);
	let client = builder.build_http();

	let res = client.request(request).await?;
	Ok(res)
}

#[js_fn]
async fn get(url: String, options: Option<RequestOptions>) -> Result<Response> {
	let url: Url = Url::from_str(&url)?;
	let options = options.unwrap_or_default();
	let set_host = options.set_host;
	let request = construct_request(&url, Method::GET, options)?;
	let response = request_internal(request, set_host).await?;
	Response::new(response, url.as_str())
}

#[js_fn]
async fn post(url: String, options: Option<RequestOptions>) -> Result<Response> {
	let url: Url = Url::from_str(&url)?;
	let options = options.unwrap_or_default();
	let set_host = options.set_host;
	let request = construct_request(&url, Method::POST, options)?;
	let response = request_internal(request, set_host).await?;
	Response::new(response, url.as_str())
}

#[js_fn]
async fn put(url: String, options: Option<RequestOptions>) -> Result<Response> {
	let url: Url = Url::from_str(&url)?;
	let options = options.unwrap_or_default();
	let set_host = options.set_host;
	let request = construct_request(&url, Method::PUT, options)?;
	let response = request_internal(request, set_host).await?;
	Response::new(response, url.as_str())
}

#[js_fn]
async fn request(resource: Resource, method: Option<String>, options: Option<RequestOptions>) -> Result<Response> {
	use crate::http::request::Request;
	match resource {
		Resource::Request(Request { req, set_host, body }) => {
			let uri = req.uri_ref().unwrap().clone();
			let url = Url::from_str(&uri.to_string())?;
			let request = req.body(Body::from(body))?;
			let response = request_internal(request, set_host).await?;
			Response::new(response, url.as_str())
		}
		Resource::String(url) => {
			let url = Url::from_str(&url)?;
			let mut method = method.ok_or_else(|| Error::new("request() requires at least 2 arguments", None))?;

			method.make_ascii_uppercase();
			let method = Method::from_str(&method)?;
			let options = options.unwrap_or_default();
			let set_host = options.set_host;
			let request = construct_request(&url, method, options)?;
			let response = request_internal(request, set_host).await?;
			Response::new(response, url.as_str())
		}
	}
}

const FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(get, 1),
	function_spec!(post, 1),
	function_spec!(put, 1),
	function_spec!(request, 1),
	JSFunctionSpec::ZERO,
];

#[derive(Default)]
pub struct Http;

impl NativeModule for Http {
	const NAME: &'static str = "http";
	const SOURCE: &'static str = include_str!("http.js");

	fn module(cx: Context) -> Option<Object> {
		let mut http = Object::new(cx);

		http.define_methods(cx, FUNCTIONS);
		Response::init_class(cx, &http);
		crate::http::request::Request::init_class(cx, &http);
		Some(http)
	}
}
