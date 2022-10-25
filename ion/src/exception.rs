/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::conversions::ConversionBehavior;
use mozjs::jsapi::{
	ESClass, ExceptionStack, ExceptionStackBehavior, ExceptionStackOrNull, GetBuiltinClass, GetPendingExceptionStack, IdentifyStandardInstance,
	JS_ClearPendingException, JS_GetPendingException, JS_IsExceptionPending, JS_SetPendingException, Rooted,
};
use mozjs::jsval::{JSVal, ObjectValue};
#[cfg(feature = "sourcemap")]
use sourcemap::SourceMap;

use crate::{Context, Error, ErrorKind, Location, Object, Stack, Value};
use crate::conversions::ToValue;
use crate::error::ThrowException;
use crate::format::{format_value, NEWLINE};

#[derive(Clone, Debug)]
pub enum Exception {
	Error(Error),
	Other(JSVal),
}

#[derive(Clone, Debug)]
pub struct ErrorReport {
	pub exception: Exception,
	pub stack: Option<Stack>,
}

impl Exception {
	/// Gets an exception from the runtime.
	/// Returns [None] if there is no pending exception.
	pub fn new(cx: &Context) -> Option<Exception> {
		unsafe {
			if JS_IsExceptionPending(**cx) {
				let mut exception = Value::undefined(cx);
				if JS_GetPendingException(**cx, exception.handle_mut().into()) {
					let exception = Exception::from_value(cx, &exception);
					Exception::clear(cx);
					Some(exception)
				} else {
					None
				}
			} else {
				None
			}
		}
	}

	pub fn from_value<'cx>(cx: &'cx Context, value: &Value<'cx>) -> Exception {
		if value.is_object() {
			let object = value.to_object(cx);
			let exception = Exception::from_object(cx, &object);
			exception
		} else {
			Exception::Other(***value)
		}
	}

	pub fn from_object<'cx>(cx: &'cx Context, exception: &Object<'cx>) -> Exception {
		unsafe {
			let mut class = ESClass::Other;
			if GetBuiltinClass(**cx, exception.handle().into(), &mut class) && class == ESClass::Error {
				let message = exception.get_as::<String>(cx, "message", ()).unwrap_or_default();
				let file = exception.get_as::<String>(cx, "fileName", ()).unwrap();
				let lineno = exception.get_as::<u32>(cx, "lineNumber", ConversionBehavior::Clamp).unwrap();
				let column = exception.get_as::<u32>(cx, "columnNumber", ConversionBehavior::Clamp).unwrap();

				let location = Location { file, lineno, column };
				let kind = ErrorKind::from_proto_key(IdentifyStandardInstance(exception.handle().get()));
				let error = Error {
					kind,
					message,
					location: Some(location),
					object: Some(exception.handle().get()),
				};
				Exception::Error(error)
			} else {
				Exception::Other(ObjectValue(exception.handle().get()))
			}
		}
	}

	/// Clears all exceptions within the runtime.
	pub fn clear(cx: &Context) {
		unsafe { JS_ClearPendingException(**cx) };
	}

	#[cfg(feature = "sourcemap")]
	pub fn transform_with_sourcemap(&mut self, sourcemap: &SourceMap) {
		if let Exception::Error(Error { location: Some(location), .. }) = self {
			if let Some(token) = sourcemap.lookup_token(location.lineno - 1, location.column - 1) {
				location.lineno = token.get_src_line() + 1;
				location.column = token.get_src_col() + 1;
			}
		}
	}

	/// Formats the exception as an error message.
	pub fn format(&self, cx: &Context) -> String {
		match self {
			Exception::Error(error) => {
				let Error { kind, message, location, .. } = error;
				if let Some(location) = location {
					let Location { file, lineno, column } = location;
					if !file.is_empty() {
						return if *lineno == 0 {
							format!("Uncaught {} at {} - {}", kind, file, message)
						} else if *column == 0 {
							format!("Uncaught {} at {}:{} - {}", kind, file, lineno, message)
						} else {
							format!("Uncaught {} at {}:{}:{} - {}", kind, file, lineno, column, message)
						};
					}
				}
				format!("Uncaught {} - {}", kind, message)
			}
			Exception::Other(value) => {
				format!(
					"Uncaught Exception - {}",
					format_value(cx, Default::default(), &cx.root_value(*value).into())
				)
			}
		}
	}
}

impl ThrowException for Exception {
	fn throw(&self, cx: &Context) {
		match self {
			Exception::Error(error) => {
				if let Error { object: Some(object), .. } = error {
					let exception = Value::from(cx.root_value(ObjectValue(*object)));
					unsafe { JS_SetPendingException(**cx, exception.handle().into(), ExceptionStackBehavior::DoNotCapture) }
				} else {
					error.throw(cx);
				}
			}
			Exception::Other(value) => {
				let value = Value::from(cx.root_value(*value));
				unsafe { JS_SetPendingException(**cx, value.handle().into(), ExceptionStackBehavior::Capture) }
			}
		}
	}
}

impl<'cx> ToValue<'cx> for Exception {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value<'cx>) {
		match self {
			Exception::Error(error) => error.to_value(cx, value),
			Exception::Other(other) => value.handle_mut().set(*other),
		}
	}
}

impl<E: Into<Error>> From<E> for Exception {
	fn from(error: E) -> Exception {
		Exception::Error(error.into())
	}
}

impl ErrorReport {
	/// Creates a new [ErrorReport] with an [Exception] from the runtime.
	/// Returns [None] if there is no pending exception.
	pub fn new(cx: &Context) -> Option<ErrorReport> {
		Exception::new(cx).map(|exception| ErrorReport { exception, stack: None })
	}

	/// Creates a new [ErrorReport] with an [Exception] and exception stack from the error.
	/// Returns [None] if there is no pending exception.
	pub fn new_with_error_stack(cx: &Context) -> Option<ErrorReport> {
		ErrorReport::new(cx).map(|report| ErrorReport::from_exception_with_error_stack(cx, report.exception))
	}

	/// Creates a new [ErrorReport] with an [Exception] and exception stack from the runtime.
	/// Returns [None] if there is no pending exception.
	pub fn new_with_exception_stack(cx: &Context) -> Option<ErrorReport> {
		unsafe {
			if JS_IsExceptionPending(**cx) {
				let mut exception_stack = ExceptionStack {
					exception_: Rooted::new_unrooted(),
					stack_: Rooted::new_unrooted(),
				};
				if GetPendingExceptionStack(**cx, &mut exception_stack) {
					let exception = Value::from(cx.root_value(exception_stack.exception_.ptr));
					let exception = Exception::from_value(cx, &exception);
					let stack = Stack::from_object(cx, exception_stack.stack_.ptr);
					Exception::clear(cx);
					Some(ErrorReport { exception, stack })
				} else {
					None
				}
			} else {
				None
			}
		}
	}

	pub fn from(exception: Exception, stack: Option<Stack>) -> ErrorReport {
		ErrorReport { exception, stack }
	}

	pub fn from_exception_with_error_stack(cx: &Context, exception: Exception) -> ErrorReport {
		let stack = if let Exception::Error(Error { object: Some(object), .. }) = exception {
			unsafe {
				rooted!(in(**cx) let exc = object);
				Stack::from_object(cx, ExceptionStackOrNull(exc.handle().into()))
			}
		} else {
			None
		};
		ErrorReport { exception, stack }
	}

	#[cfg(feature = "sourcemap")]
	pub fn transform_with_sourcemap(&mut self, sourcemap: &SourceMap) {
		self.exception.transform_with_sourcemap(sourcemap);
		if let Some(stack) = &mut self.stack {
			stack.transform_with_sourcemap(sourcemap)
		}
	}

	pub fn format(&self, cx: &Context) -> String {
		let mut string = self.exception.format(cx);
		if let Some(ref stack) = self.stack {
			if !stack.is_empty() {
				string.push_str(NEWLINE);
				string.push_str(&stack.format());
			}
		}
		string
	}
}
