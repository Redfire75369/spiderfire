/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::*;

pub(crate) fn report_and_clear_exception(cx: *mut JSContext) {
	println!("Script Execution Failed :(");
	unsafe {
		if JS_IsExceptionPending(cx) {
			capture_stack!(in(cx) let stack);
			let js_stack = stack.and_then(|s| s.as_string(None, StackFormat::Default));
			if let Some(stack_trace) = js_stack {
				println!("{}", stack_trace);
			}

			JS_ClearPendingException(cx);
		}
	}
}
