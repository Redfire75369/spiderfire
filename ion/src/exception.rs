/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};
use std::ptr;

use mozjs::conversions::{ConversionBehavior, jsstr_to_string};
use mozjs::jsapi::{
	BuildStackString, ESClass, ExceptionStack, ExceptionStackOrNull, GetBuiltinClass, GetPendingExceptionStack, IdentifyStandardInstance,
	JS_ClearPendingException, JS_GetPendingException, JS_IsExceptionPending, JSObject, JSProtoKey, JSString, StackFormat,
};
use mozjs::jsval::{JSVal, UndefinedValue};
use mozjs_sys::jsgc::Rooted;
#[cfg(feature = "sourcemap")]
use sourcemap::SourceMap;

use crate::{Context, Object};
use crate::format::{format_value, INDENT, NEWLINE};
use crate::utils::normalise_path;

#[derive(Clone, Debug)]
pub enum Exception {
	Error {
		kind: Option<String>,
		message: String,
		file: String,
		lineno: u32,
		column: u32,
		object: Object,
	},
	Other(JSVal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackRecord {
	pub function: Option<String>,
	pub file: String,
	pub lineno: u32,
	pub column: u32,
}

#[derive(Clone, Debug)]
pub struct Stack {
	pub records: Vec<StackRecord>,
}

#[derive(Clone, Debug)]
pub struct ErrorReport {
	pub exception: Exception,
	pub stack: Option<Stack>,
}

impl Exception {
	/// Gets an exception from the runtime.
	/// Returns [None] if there is no pending exception.
	pub fn new(cx: Context) -> Option<Exception> {
		unsafe {
			if JS_IsExceptionPending(cx) {
				rooted!(in(cx) let mut exception = UndefinedValue());
				if JS_GetPendingException(cx, exception.handle_mut().into()) {
					let exception = Exception::from_value(cx, exception.get());
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

	pub fn from_value(cx: Context, value: JSVal) -> Exception {
		if value.is_object() {
			Exception::from_object(cx, Object::from(value.to_object()))
		} else {
			Exception::Other(value)
		}
	}

	pub fn from_object(cx: Context, exception: Object) -> Exception {
		unsafe {
			rooted!(in(cx) let exc = *exception);

			let mut class = ESClass::Other;
			if GetBuiltinClass(cx, exc.handle().into(), &mut class) && class == ESClass::Error {
				let message = exception.get_as::<String>(cx, "message", ()).unwrap();
				let filename = exception.get_as::<String>(cx, "fileName", ()).unwrap();
				let lineno = exception.get_as::<u32>(cx, "lineNumber", ConversionBehavior::Clamp).unwrap();
				let column = exception.get_as::<u32>(cx, "columnNumber", ConversionBehavior::Clamp).unwrap();

				let kind = IdentifyStandardInstance(*exception);

				use JSProtoKey::{
					JSProto_Error, JSProto_InternalError, JSProto_AggregateError, JSProto_EvalError, JSProto_RangeError, JSProto_ReferenceError,
					JSProto_SyntaxError, JSProto_TypeError, JSProto_CompileError, JSProto_LinkError, JSProto_RuntimeError,
				};
				let kind = match kind {
					JSProto_Error => Some("Error"),
					JSProto_InternalError => Some("InternalError"),
					JSProto_AggregateError => Some("AggregateError"),
					JSProto_EvalError => Some("EvalError"),
					JSProto_RangeError => Some("RangeError"),
					JSProto_ReferenceError => Some("ReferenceError"),
					JSProto_SyntaxError => Some("SyntaxError"),
					JSProto_TypeError => Some("TypeError"),
					JSProto_CompileError => Some("CompileError"),
					JSProto_LinkError => Some("LinkError"),
					JSProto_RuntimeError => Some("RuntimeError"),
					_ => None,
				};

				Exception::Error {
					kind: kind.map(String::from),
					message,
					file: filename,
					lineno,
					column,
					object: exception,
				}
			} else {
				Exception::Other(exception.to_value())
			}
		}
	}

	/// Clears all exceptions within the runtime.
	pub fn clear(cx: Context) {
		unsafe { JS_ClearPendingException(cx) };
	}

	#[cfg(feature = "sourcemap")]
	pub fn transform_with_sourcemap(&mut self, sourcemap: &SourceMap) {
		if let Exception::Error { lineno, column, .. } = self {
			if let Some(token) = sourcemap.lookup_token(*lineno - 1, *column - 1) {
				*lineno = token.get_src_line() + 1;
				*column = token.get_src_col() + 1;
			}
		}
	}

	/// Formats the exception as an error message.
	pub fn format(&self, cx: Context) -> String {
		match self {
			Exception::Error { kind, message, file, lineno, column, .. } => {
				let kind = kind.as_ref().map(|k| k.to_string()).unwrap_or_else(|| String::from("Exception"));
				if !file.is_empty() {
					if lineno == &0 {
						format!("Uncaught {} at {} - {}", kind, file, message)
					} else if column == &0 {
						format!("Uncaught {} at {}:{} - {}", kind, file, lineno, message)
					} else {
						format!("Uncaught {} at {}:{}:{} - {}", kind, file, lineno, column, message)
					}
				} else {
					format!("Uncaught {} - {}", kind, message)
				}
			}
			Exception::Other(value) => {
				format!("Uncaught Exception - {}", format_value(cx, Default::default(), *value))
			}
		}
	}
}

impl StackRecord {
	#[cfg(feature = "sourcemap")]
	pub fn transform_with_sourcemap(&mut self, sourcemap: &SourceMap) {
		if self.lineno != 0 && self.column != 0 {
			if let Some(token) = sourcemap.lookup_token(self.lineno - 1, self.column - 1) {
				self.lineno = token.get_src_line() + 1;
				self.column = token.get_src_col() + 1;
			}
		}
	}
}

impl Display for StackRecord {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str(self.function.as_deref().unwrap_or(""))?;
		f.write_str("@")?;
		f.write_str(&self.file)?;
		f.write_str(":")?;
		f.write_str(&self.lineno.to_string())?;
		f.write_str(":")?;
		f.write_str(&self.column.to_string())?;
		Ok(())
	}
}

impl Stack {
	pub fn from_string(string: &str) -> Stack {
		let mut records = Vec::new();
		for line in string.lines() {
			let (function, line) = line.split_once('@').unwrap();
			let (line, column) = line.rsplit_once(':').unwrap();
			let (file, lineno) = line.rsplit_once(':').unwrap();

			let function = if function.is_empty() { None } else { Some(String::from(function)) };
			records.push(StackRecord {
				function,
				file: String::from(normalise_path(file).to_str().unwrap()),
				lineno: lineno.parse().unwrap(),
				column: column.parse().unwrap(),
			});
		}
		Stack { records }
	}

	pub fn from_object(cx: Context, stack: *mut JSObject) -> Option<Stack> {
		unsafe {
			rooted!(in(cx) let stack = stack);
			rooted!(in(cx) let mut string: *mut JSString);

			if BuildStackString(
				cx,
				ptr::null_mut(),
				stack.handle().into(),
				string.handle_mut().into(),
				0,
				StackFormat::SpiderMonkey,
			) {
				let string = jsstr_to_string(cx, string.get());
				Some(Stack::from_string(&string))
			} else {
				None
			}
		}
	}

	pub fn from_capture(cx: Context) -> Option<Stack> {
		unsafe {
			capture_stack!(in(cx) let stack);
			stack.and_then(|stack| stack.as_string(None, StackFormat::SpiderMonkey).as_deref().map(Stack::from_string))
		}
	}

	pub fn is_empty(&self) -> bool {
		self.records.is_empty()
	}

	#[cfg(feature = "sourcemap")]
	pub fn transform_with_sourcemap(&mut self, sourcemap: &SourceMap) {
		for record in &mut self.records {
			record.transform_with_sourcemap(sourcemap);
		}
	}

	pub fn format(&self) -> String {
		let mut string = String::from("");
		for record in &self.records {
			string.push_str(INDENT);
			string.push_str(&record.to_string());
			string.push_str(NEWLINE);
		}
		string.pop();
		string
	}
}

impl Display for Stack {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str(&self.format())
	}
}

impl ErrorReport {
	/// Creates a new [ErrorReport] with an [Exception] from the runtime.
	/// Returns [None] if there is no pending exception.
	pub fn new(cx: Context) -> Option<ErrorReport> {
		Exception::new(cx).map(|exception| ErrorReport { exception, stack: None })
	}

	/// Creates a new [ErrorReport] with an [Exception] and exception stack from the error.
	/// Returns [None] if there is no pending exception.
	pub fn new_with_error_stack(cx: Context) -> Option<ErrorReport> {
		ErrorReport::new(cx).map(|report| ErrorReport::from_exception_with_error_stack(cx, report.exception))
	}

	/// Creates a new [ErrorReport] with an [Exception] and exception stack from the runtime.
	/// Returns [None] if there is no pending exception.
	pub fn new_with_exception_stack(cx: Context) -> Option<ErrorReport> {
		unsafe {
			if JS_IsExceptionPending(cx) {
				let mut exception_stack = ExceptionStack {
					exception_: Rooted::new_unrooted(),
					stack_: Rooted::new_unrooted(),
				};
				if GetPendingExceptionStack(cx, &mut exception_stack) {
					let exception = Exception::from_value(cx, exception_stack.exception_.ptr);
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

	pub fn from_exception_with_error_stack(cx: Context, exception: Exception) -> ErrorReport {
		let stack = if let Exception::Error { object, .. } = exception {
			unsafe {
				rooted!(in(cx) let exc = *object);
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

	pub fn format(&self, cx: Context) -> String {
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
