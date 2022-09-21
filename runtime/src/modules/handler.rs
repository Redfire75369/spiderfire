/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use mozjs::jsapi::{ExceptionStackOrNull, JSFunctionSpec};
use mozjs::jsval::JSVal;

use ion::{Context, ErrorReport, Exception, Function, Promise};
use ion::exception::{parse_stack, stack_to_string};

use crate::cache::map::transform_error_report_with_sourcemaps;

#[js_fn]
unsafe fn on_rejected(cx: Context, value: JSVal) {
	let exception = Exception::from_value(cx, value);
	let stack = if value.is_object() {
		rooted!(in(cx) let rval = value.to_object());
		ExceptionStackOrNull(rval.handle().into())
	} else {
		ptr::null_mut()
	};

	let stack = stack_to_string(cx, stack).map(|s| parse_stack(&s));
	let mut report = ErrorReport::from(exception, stack);
	Exception::clear(cx);
	transform_error_report_with_sourcemaps(&mut report);
	println!("{}", report.format(cx));
}

static ON_REJECTED: JSFunctionSpec = function_spec!(on_rejected, "onRejected", 0);

pub fn add_handler_reactions(cx: Context, mut promise: Promise) -> bool {
	let on_rejected = Function::from_spec(cx, &ON_REJECTED);
	promise.add_reactions(cx, None, Some(on_rejected))
}
