/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use hyper::Method;
use mozjs::jsapi::JSFunctionSpec;

use ion::{ClassInitialiser, Context, Object, ResultExc};
use runtime::modules::NativeModule;

use crate::http::{Headers, Request, Resource, Response};
use crate::http::client::{default_client, GLOBAL_CLIENT};
use crate::http::client::Client;
use crate::http::network::request_internal;
use crate::http::request::{RequestBuilderOptions, RequestOptions};

#[js_fn]
async fn get(url: String, options: Option<RequestOptions>) -> ResultExc<Response> {
	let options = RequestBuilderOptions::from_request_options(options, Method::GET.to_string());
	let request = Request::constructor(Resource::String(url), Some(options))?;

	request_internal(request).await
}

#[js_fn]
async fn post(url: String, options: Option<RequestOptions>) -> ResultExc<Response> {
	let options = RequestBuilderOptions::from_request_options(options, Method::POST.to_string());
	let request = Request::constructor(Resource::String(url), Some(options))?;

	request_internal(request).await
}

#[js_fn]
async fn put(url: String, options: Option<RequestOptions>) -> ResultExc<Response> {
	let options = RequestBuilderOptions::from_request_options(options, Method::PUT.to_string());
	let request = Request::constructor(Resource::String(url), Some(options))?;

	request_internal(request).await
}

#[js_fn]
async fn request(resource: Resource, method: Option<String>, options: Option<RequestOptions>) -> ResultExc<Response> {
	use crate::http::request::Request;
	match resource {
		Resource::Request(request) => request_internal(request).await,
		Resource::String(url) => {
			let options = RequestBuilderOptions::from_request_options(options, method);
			let request = Request::constructor(Resource::String(url), Some(options))?;

			request_internal(request).await
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

	fn module<'cx>(cx: &'cx Context) -> Option<Object<'cx>> {
		let mut http = Object::new(cx);

		http.define_methods(cx, FUNCTIONS);
		Headers::init_class(cx, &mut http);
		Request::init_class(cx, &mut http);
		Response::init_class(cx, &mut http);

		Client::init_class(cx, &mut http);
		let _ = GLOBAL_CLIENT.set(default_client());

		Some(http)
	}
}
