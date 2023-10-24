/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::rust::IntoHandle;

pub use client::{default_client, GLOBAL_CLIENT};
pub use header::Headers;
use ion::{ClassDefinition, Context, Local, Object, Promise, ResultExc};
use ion::flags::PropertyFlags;
pub use network::request_internal;
pub use request::{Request, RequestInfo, RequestInit};
pub use response::Response;

use crate::promise::future_to_promise;

mod body;
mod client;
mod header;
mod network;
mod request;
mod response;

// TODO: Specification-Compliant Fetch Implementation
#[js_fn]
fn fetch<'cx>(cx: &'cx Context, resource: RequestInfo, init: Option<RequestInit>) -> ResultExc<Promise<'cx>> {
	let request = Request::constructor(cx, resource, init)?;
	let request = cx.root_persistent_object(Request::new_object(cx, Box::new(request)));
	let cx2 = unsafe { Context::new_unchecked(cx.as_ptr()) };
	let request = request.handle().into_handle();
	Ok(future_to_promise(cx, async move {
		let request = Object::from(unsafe { Local::from_raw_handle(request) });
		let res = request_internal(&cx2, &request, GLOBAL_CLIENT.get().unwrap().clone()).await;
		cx2.unroot_persistent_object(request.handle().get());
		res
	}))
}

pub fn define(cx: &Context, global: &mut Object) -> bool {
	let _ = GLOBAL_CLIENT.set(default_client());
	global.define_method(cx, "fetch", fetch, 1, PropertyFlags::CONSTANT_ENUMERATED);
	Headers::init_class(cx, global).0 && Request::init_class(cx, global).0 && Response::init_class(cx, global).0
}
