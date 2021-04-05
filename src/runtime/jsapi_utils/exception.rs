/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::ffi::CStr;
use ::std::slice::from_raw_parts;

use libc::c_uint;
use mozjs::jsapi::*;
use mozjs::jsval::UndefinedValue;

pub(crate) struct ErrorInfo {
	message: String,
	filename: String,
	lineno: c_uint,
	column: c_uint,
	#[allow(dead_code)]
	stack: Option<String>,
}

impl ErrorInfo {
	pub(crate) fn format(&self) -> String {
		format!(
			"Uncaught exception at {}:{}:{} - {}",
			self.filename, self.lineno, self.column, self.message
		)
	}
}

pub(crate) fn report_and_clear_exception(cx: *mut JSContext) {
	rooted!(in(cx) let mut exception = UndefinedValue());

	unsafe {
		if !JS_GetPendingException(cx, exception.handle_mut().into()) {
			return;
		}
		JS_ClearPendingException(cx);

		let exception_handle = Handle::from_marked_location(&exception.get().to_object());
		let report = JS_ErrorFromException(cx, exception_handle);

		if let Some(error_info) = error_info_from_exception(report) {
			println!("{}", error_info.format());

			capture_stack!(in(cx) let stack);
			let str_stack = stack.unwrap().as_string(None, StackFormat::SpiderMonkey).unwrap();
			println!("{}", str_stack);
		}
	}
}

unsafe fn error_info_from_exception(report: *mut JSErrorReport) -> Option<ErrorInfo> {
	if report.is_null() {
		return None;
	}

	let message = {
		let message = (*report)._base.message_.data_ as *const u8;
		let length = (0..).find(|i| *message.offset(*i) == 0).unwrap();
		let message = from_raw_parts(message, length as usize);
		String::from(String::from_utf8_lossy(message))
	};

	let filename = {
		let filename = (*report)._base.filename;
		if filename.is_null() {
			"<anonymous>".to_string()
		} else {
			CStr::from_ptr(filename).to_string_lossy().into_owned()
		}
	};

	let lineno = (*report)._base.lineno;
	let column = (*report)._base.column;

	let error = ErrorInfo {
		message,
		filename,
		lineno,
		column,
		stack: None,
	};

	Some(error)
}
