/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::ffi::CStr;
use ::std::slice::from_raw_parts;

use mozjs::jsapi::*;
use mozjs::jsval::UndefinedValue;

pub(crate) fn report_and_clear_exception(cx: *mut JSContext) {
	rooted!(in(cx) let mut exception = UndefinedValue());

	unsafe {
		if !JS_GetPendingException(cx, exception.handle_mut().into()) {
			return;
		}
		JS_ClearPendingException(cx);

		let exception_handle = Handle::from_marked_location(&exception.get().to_object());
		let report = JS_ErrorFromException(cx, exception_handle);

		if report.is_null() {
			return;
		}

		let filename = {
			let filename = (*report)._base.filename;
			if filename.is_null() {
				"<anonymous>".to_string()
			} else {
				CStr::from_ptr(filename).to_string_lossy().into_owned()
			}
		};

		let message = {
			let message = (*report)._base.message_.data_ as *const u8;
			let length = (0..).find(|i| *message.offset(*i) == 0).unwrap();
			let message = from_raw_parts(message, length as usize);
			String::from_utf8_lossy(message)
		};

		let error_name = {
			let error_name = (*report)._base.errorMessageName;
			if error_name.is_null() {
				"Error".to_string()
			} else {
				CStr::from_ptr(error_name).to_string_lossy().into_owned()
			}
		};

		println!("Uncaught {} at {}:{}:{}: {}", error_name, filename, (*report)._base.lineno, (*report)._base.column, message);
		capture_stack!(in(cx) let stack);
		let str_stack = stack.unwrap().as_string(None, StackFormat::SpiderMonkey).unwrap();
		println!("{}", str_stack);
	}
}
