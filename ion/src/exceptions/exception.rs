/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::*;
use mozjs::jsval::UndefinedValue;

use crate::exceptions::report::ErrorReport;
use crate::functions::macros::IonContext;

pub unsafe fn report_and_clear_exception(cx: IonContext) -> bool {
	rooted!(in(cx) let mut exception = UndefinedValue());
	if !JS_GetPendingException(cx, exception.handle_mut().into()) {
		return false;
	}
	JS_ClearPendingException(cx);

	let exception_handle = Handle::from_marked_location(&exception.get().to_object());
	if let Some(report) = ErrorReport::new_with_stack(cx, JS_ErrorFromException(cx, exception_handle)) {
		print_error_report(report);
		true
	} else {
		false
	}
}

pub fn print_error_report(report: ErrorReport) {
	println!("{}", report.format());
	if let Some(stack) = report.stack() {
		println!("{}", stack);
	}
}
