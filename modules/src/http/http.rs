/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use hyper::client::HttpConnector;
use hyper::Method;
use hyper_rustls::HttpsConnector;
use mozjs::jsapi::JSFunctionSpec;

use ion::{ClassDefinition, Context, Object, ResultExc};
use runtime::globals::fetch::{default_client, GLOBAL_CLIENT, Headers, Request, request_internal, RequestBuilderInit, RequestInit, Resource, Response};
use runtime::modules::NativeModule;

use crate::http::client::{Client, ClientRequestOptions};

#[derive(Default, FromValue)]
pub struct RequestClientInit {
	client: ClientRequestOptions,
	#[ion(default, inherit)]
	init: RequestInit,
}

fn to_client(init: Option<&RequestClientInit>) -> hyper::client::Client<HttpsConnector<HttpConnector>> {
	init.map(|init| init.client.to_client())
		.unwrap_or_else(|| GLOBAL_CLIENT.get().unwrap().clone())
}

#[js_fn]
async fn get(url: String, init: Option<RequestClientInit>) -> ResultExc<Response> {
	let client = to_client(init.as_ref());
	let options = RequestBuilderInit::from_request_init(init.map(|opt| opt.init), Method::GET.to_string());
	let request = Request::constructor(Resource::String(url), Some(options))?;

	request_internal(request, client).await
}

#[js_fn]
async fn post(url: String, init: Option<RequestClientInit>) -> ResultExc<Response> {
	let client = to_client(init.as_ref());
	let options = RequestBuilderInit::from_request_init(init.map(|opt| opt.init), Method::POST.to_string());
	let request = Request::constructor(Resource::String(url), Some(options))?;

	request_internal(request, client).await
}

#[js_fn]
async fn put(url: String, init: Option<RequestClientInit>) -> ResultExc<Response> {
	let client = to_client(init.as_ref());
	let options = RequestBuilderInit::from_request_init(init.map(|opt| opt.init), Method::PUT.to_string());
	let request = Request::constructor(Resource::String(url), Some(options))?;

	request_internal(request, client).await
}

#[js_fn]
async fn request(resource: Resource, method: Option<String>, init: Option<RequestClientInit>) -> ResultExc<Response> {
	let client = to_client(init.as_ref());
	match resource {
		Resource::Request(request) => request_internal(request, client).await,
		Resource::String(url) => {
			let options = RequestBuilderInit::from_request_init(init.map(|opt| opt.init), method);
			let request = Request::constructor(Resource::String(url), Some(options))?;

			request_internal(request, client).await
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
		let global = Object::global(cx);

		http.define_methods(cx, FUNCTIONS);
		Client::init_class(cx, &mut http);
		GLOBAL_CLIENT.get_or_init(default_client);

		if let Some(headers) = global.get(cx, stringify!(Headers)) {
			http.set(cx, stringify!(Headers), &headers);
		} else {
			Headers::init_class(cx, &mut http);
		}

		if let Some(request) = global.get(cx, stringify!(Request)) {
			http.set(cx, stringify!(Request), &request);
		} else {
			Request::init_class(cx, &mut http);
		}

		if let Some(response) = global.get(cx, stringify!(Response)) {
			http.set(cx, stringify!(Response), &response);
		} else {
			Response::init_class(cx, &mut http);
		}

		Some(http)
	}
}
