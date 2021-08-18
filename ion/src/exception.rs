/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::conversions::ConversionBehavior;
use mozjs::jsapi::{JS_ClearPendingException, JS_GetPendingException, JS_IsExceptionPending, StackFormat};
use mozjs::jsval::UndefinedValue;

use crate::IonContext;
use crate::objects::object::IonObject;

#[derive(Clone, Debug)]
pub struct Exception {
	message: String,
	filename: String,
	lineno: u32,
	column: u32,
}

#[derive(Clone, Debug)]
pub struct ErrorReport {
	exception: Exception,
	stack: Option<String>,
}

impl Exception {
	pub unsafe fn new(cx: IonContext) -> Option<Exception> {
		if JS_IsExceptionPending(cx) {
			rooted!(in(cx) let mut exception = UndefinedValue());
			if JS_GetPendingException(cx, exception.handle_mut().into()) {
				let exception = IonObject::from(exception.to_object());
				Exception::clear(cx);

				let message = exception.get_as::<String>(cx, String::from("message"), ()).unwrap();
				let filename = exception.get_as::<String>(cx, String::from("fileName"), ()).unwrap();
				let lineno = exception
					.get_as::<u32>(cx, String::from("lineNumber"), ConversionBehavior::Clamp)
					.unwrap();
				let column = exception
					.get_as::<u32>(cx, String::from("columnNumber"), ConversionBehavior::Clamp)
					.unwrap();

				Some(Exception {
					message,
					filename,
					lineno,
					column,
				})
			} else {
				None
			}
		} else {
			None
		}
	}

	pub unsafe fn clear(cx: IonContext) {
		JS_ClearPendingException(cx);
	}
}

impl ErrorReport {
	pub fn new(exception: Exception) -> ErrorReport {
		ErrorReport { exception, stack: None }
	}

	pub fn new_with_stack(cx: IonContext, exception: Exception) -> ErrorReport {
		unsafe {
			capture_stack!(in(cx) let stack);
			let stack = stack.unwrap().as_string(None, StackFormat::SpiderMonkey);
			ErrorReport { exception, stack }
		}
	}

	pub fn stack(&self) -> Option<&String> {
		self.stack.as_ref()
	}

	pub fn format(&self) -> String {
		format!(
			"Uncaught exception at {}:{}:{} - {}",
			self.exception.filename, self.exception.lineno, self.exception.column, self.exception.message
		)
	}

	pub fn print(&self) {
		println!("{}", self.format());
		if let Some(stack) = self.stack() {
			println!("{}", stack);
		}
	}
}
