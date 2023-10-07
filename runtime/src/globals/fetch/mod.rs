/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub use client::{default_client, GLOBAL_CLIENT};
pub use header::Headers;
use ion::{ClassDefinition, Context, Object, ResultExc};
use ion::flags::PropertyFlags;
pub use network::request_internal;
pub use request::{Request, RequestBuilderInit, RequestInit, Resource};
pub use response::Response;

mod body;
mod client;
mod header;
mod network;
mod request;
mod response;

// TODO: Specification-Compliant Fetch Implementation
#[js_fn]
#[ion(runtime = crate)]
async fn fetch(resource: Resource, init: Option<RequestBuilderInit>) -> ResultExc<Response> {
	let request = Request::constructor(resource, init)?;
	request_internal(request, GLOBAL_CLIENT.get().unwrap().clone()).await
}

pub fn define(cx: &Context, global: &mut Object) -> bool {
	let _ = GLOBAL_CLIENT.set(default_client());
	global.define_method(cx, "fetch", fetch, 1, PropertyFlags::CONSTANT_ENUMERATED);
	Headers::init_class(cx, global).0 && Request::init_class(cx, global).0 && Response::init_class(cx, global).0
}
