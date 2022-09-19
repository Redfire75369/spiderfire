/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::JSFunctionSpec;
use mozjs::jsval::JSVal;

use ion::{Context, Function, Promise};
use ion::format::format_value;

#[js_fn]
fn on_rejected(cx: Context, value: JSVal) {
	println!("Uncaught Exception: {}", format_value(cx, Default::default(), value));
}

static ON_REJECTED: JSFunctionSpec = function_spec!(on_rejected, "onRejected", 0);

pub fn add_handler_reactions(cx: Context, mut promise: Promise) -> bool {
	let on_rejected = Function::from_spec(cx, &ON_REJECTED);
	promise.add_reactions(cx, None, Some(on_rejected))
}
