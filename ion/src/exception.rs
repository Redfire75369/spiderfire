/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Display, Formatter};
use std::ptr;

use mozjs::conversions::{ConversionBehavior, jsstr_to_string};
use mozjs::jsapi::{
	BuildStackString, ESClass, ExceptionStack, GetBuiltinClass, GetPendingExceptionStack, IdentifyStandardInstance, JS_ClearPendingException,
	JS_GetPendingException, JS_IsExceptionPending, JSObject, JSProtoKey, JSString, StackFormat,
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
pub struct ErrorReport {
	pub exception: Exception,
	pub stack: Option<Vec<StackRecord>>,
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
			Exception::Error { kind, file, lineno, column, message } => {
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

impl ErrorReport {
	/// Creates a new [ErrorReport] with an [Exception] from the runtime.
	/// Returns [None] if there is no pending exception.
	pub fn new(cx: Context) -> Option<ErrorReport> {
		Exception::new(cx).map(|exception| ErrorReport { exception, stack: None })
	}

	/// Creates a new [ErrorReport] with an [Exception] and exception stack from the runtime.
	/// Returns [None] if there is no pending exception.
	pub fn new_with_stack(cx: Context) -> Option<ErrorReport> {
		unsafe {
			if JS_IsExceptionPending(cx) {
				let mut exception_stack = ExceptionStack {
					exception_: Rooted::new_unrooted(),
					stack_: Rooted::new_unrooted(),
				};
				if GetPendingExceptionStack(cx, &mut exception_stack) {
					let exception = Exception::from_value(cx, exception_stack.exception_.ptr);
					let stack = stack_to_string(cx, exception_stack.stack_.ptr).map(|s| parse_stack(&s));
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

	pub fn from(exception: Exception, stack: Option<Vec<StackRecord>>) -> ErrorReport {
		ErrorReport { exception, stack }
	}

	#[cfg(feature = "sourcemap")]
	pub fn transform_with_sourcemap(&mut self, sourcemap: &SourceMap) {
		self.exception.transform_with_sourcemap(sourcemap);
		if let Some(stack) = &mut self.stack {
			for record in stack {
				record.transform_with_sourcemap(sourcemap);
			}
		}
	}

	pub fn format(&self, cx: Context) -> String {
		let mut str = self.exception.format(cx);
		if let Some(ref stack) = self.stack {
			if !stack.is_empty() {
				str.push_str(NEWLINE);
				str.push_str(&format_stack(stack));
			}
		}
		str
	}
}

impl Display for StackRecord {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

pub fn stack_to_string(cx: Context, stack: *mut JSObject) -> Option<String> {
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
			Some(jsstr_to_string(cx, string.get()))
		} else {
			None
		}
	}
}

pub fn parse_stack(string: &str) -> Vec<StackRecord> {
	let mut stack = Vec::new();
	for line in string.lines() {
		let (function, line) = line.split_once('@').unwrap();
		let (line, column) = line.rsplit_once(':').unwrap();
		let (file, lineno) = line.rsplit_once(':').unwrap();

		let function = if function.is_empty() { None } else { Some(String::from(function)) };
		stack.push(StackRecord {
			function,
			file: String::from(normalise_path(file).to_str().unwrap()),
			lineno: lineno.parse().unwrap(),
			column: column.parse().unwrap(),
		});
	}
	stack
}

pub fn format_stack(stack: &[StackRecord]) -> String {
	let mut string = String::from("");
	for record in stack {
		string.push_str(&INDENT.repeat(2));
		string.push_str(&record.to_string());
		string.push_str(NEWLINE);
	}
	string.pop();
	string
}
