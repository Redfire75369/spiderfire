/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::ffi::CStr;
use ::std::slice::from_raw_parts;

use libc::c_uint;
use mozjs::jsapi::*;

pub struct ErrorReport {
	message: String,
	filename: String,
	lineno: c_uint,
	column: c_uint,
	stack: Option<String>,
}

impl ErrorReport {
	pub unsafe fn new(report: *mut JSErrorReport) -> Option<ErrorReport> {
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

		let error = ErrorReport {
			message,
			filename,
			lineno,
			column,
			stack: None,
		};

		Some(error)
	}

	pub unsafe fn new_with_stack(cx: *mut JSContext, report: *mut JSErrorReport) -> Option<ErrorReport> {
		if let Some(error) = ErrorReport::new(report) {
			capture_stack!(in(cx) let stack);
			let str_stack = stack.unwrap().as_string(None, StackFormat::SpiderMonkey).unwrap();

			let report = ErrorReport {
				message: error.message,
				filename: error.filename,
				lineno: error.lineno,
				column: error.column,
				stack: Some(str_stack),
			};
			Some(report)
		} else {
			None
		}
	}

	pub fn stack(&self) -> Option<&String> {
		self.stack.as_ref()
	}

	pub fn format(&self) -> String {
		format!(
			"Uncaught exception at {}:{}:{} - {}",
			self.filename, self.lineno, self.column, self.message
		)
	}
}
