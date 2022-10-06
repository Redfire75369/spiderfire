/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::str::FromStr;

use hyper::{Body, Method, Uri};
use mozjs::jsapi::JSFunctionSpec;
use url::Url;

use ion::{ClassInitialiser, Context, Error, Object, Result};
use runtime::modules::NativeModule;

use crate::http::client::{ClientRequestOptions, default_client, GLOBAL_CLIENT};
use crate::http::client::Client;
use crate::http::header::Headers;
use crate::http::options::RequestOptions;
use crate::http::request::{add_authorisation_header, add_host_header, check_method_with_body, Request, Resource};
use crate::http::response::Response;

fn construct_request(url: &Url, method: Method, options: RequestOptions) -> Result<hyper::Request<Body>> {
	let uri = Uri::from_str(url.as_str())?;

	check_method_with_body(&method, options.body.is_some())?;

	let mut request = hyper::Request::builder().method(method).uri(uri);

	if let Some(headers) = request.headers_mut() {
		*headers = options.headers.into_headers()?.inner();

		add_authorisation_header(headers, url, options.auth)?;
		add_host_header(headers, url, options.set_host)?;
	}

	Ok(request.body(Body::from(options.body.unwrap_or_default()))?)
}

async fn request_internal(client: ClientRequestOptions, request: hyper::Request<Body>) -> Result<hyper::Response<Body>> {
	let client = client.into_client();
	let res = client.request(request).await?;
	Ok(res)
}

#[js_fn]
async fn get(url: String, options: Option<RequestOptions>) -> Result<Response> {
	let url: Url = Url::from_str(&url)?;
	let options = options.unwrap_or_default();
	let client = options.client.clone();

	let request = construct_request(&url, Method::GET, options)?;
	let response = request_internal(client, request).await?;
	Response::new(response, url.as_str())
}

#[js_fn]
async fn post(url: String, options: Option<RequestOptions>) -> Result<Response> {
	let url: Url = Url::from_str(&url)?;
	let options = options.unwrap_or_default();
	let client = options.client.clone();

	let request = construct_request(&url, Method::POST, options)?;
	let response = request_internal(client, request).await?;
	Response::new(response, url.as_str())
}

#[js_fn]
async fn put(url: String, options: Option<RequestOptions>) -> Result<Response> {
	let url: Url = Url::from_str(&url)?;
	let options = options.unwrap_or_default();
	let client = options.client.clone();

	let request = construct_request(&url, Method::PUT, options)?;
	let response = request_internal(client, request).await?;
	Response::new(response, url.as_str())
}

#[js_fn]
async fn request(resource: Resource, method: Option<String>, options: Option<RequestOptions>) -> Result<Response> {
	use crate::http::request::Request;
	match resource {
		Resource::Request(Request { mut request, body, client }) => {
			let url = Url::from_str(&request.uri().to_string())?;
			*request.body_mut() = Body::from(body);
			let response = request_internal(client, request).await?;
			Response::new(response, url.as_str())
		}
		Resource::String(url) => {
			let url = Url::from_str(&url)?;
			let mut method = method.ok_or_else(|| Error::new("request() requires at least 2 arguments", None))?;
			method.make_ascii_uppercase();
			let method = Method::from_str(&method)?;
			let options = options.unwrap_or_default();
			let client = options.client.clone();

			let request = construct_request(&url, method, options)?;
			let response = request_internal(client, request).await?;
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
		Headers::init_class(cx, &http);
		Request::init_class(cx, &http);
		Response::init_class(cx, &http);
		Client::init_class(cx, &http);

		let _ = GLOBAL_CLIENT.set(default_client());
		Some(http)
	}
}
