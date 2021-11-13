/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Display, Formatter};

use mozjs::conversions::ConversionBehavior;
use mozjs::jsapi::{JS_ClearPendingException, JS_GetPendingException, JS_IsExceptionPending, StackFormat};
use mozjs::jsval::UndefinedValue;

use crate::IonContext;
use crate::objects::object::IonObject;

#[derive(Clone, Debug)]
pub struct Exception {
	pub message: String,
	pub filename: String,
	pub lineno: u32,
	pub column: u32,
}

#[derive(Clone, Debug)]
pub struct ErrorReport {
	pub exception: Exception,
	pub stack: Option<String>,
}

impl Exception {
	/// Gets an exception from the runtime.
	///
	/// Returns [None] is no exception is pending.
	pub fn new(cx: IonContext) -> Option<Exception> {
		unsafe {
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
	}

	/// Clears all exceptions within the runtime.
	pub unsafe fn clear(cx: IonContext) {
		JS_ClearPendingException(cx);
	}

	/// Formats the exception as an error message.
	pub fn format(&self) -> String {
		format!(
			"Uncaught exception at {}:{}:{} - {}",
			self.filename, self.lineno, self.column, self.message
		)
	}
}

impl ErrorReport {
	/// Creates a new [ErrorReport] with the given [Exception] and no stack.
	pub fn new(exception: Exception) -> ErrorReport {
		ErrorReport { exception, stack: None }
	}

	/// Creates a new [ErrorReport] with the given [Exception] and the current stack.
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

	/// Prints a formatted error message.
	pub fn print(&self) {
		println!("{}", self);
	}
}

impl Display for ErrorReport {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.exception.format())?;
		if let Some(stack) = self.stack() {
			f.write_str(&format!("\n{}", stack))?;
		}
		Ok(())
	}
}
