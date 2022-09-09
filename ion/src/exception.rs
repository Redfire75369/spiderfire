/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Display, Formatter};

use mozjs::conversions::ConversionBehavior;
use mozjs::jsapi::{JS_ClearPendingException, JS_GetPendingException, JS_IsExceptionPending, StackFormat};
use mozjs::jsval::UndefinedValue;
#[cfg(feature = "sourcemap")]
use sourcemap::SourceMap;

use crate::{Context, Object};
use crate::format::NEWLINE;
use crate::utils::normalise_path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exception {
	pub message: String,
	pub file: String,
	pub lineno: u32,
	pub column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackRecord {
	pub function: Option<String>,
	pub file: String,
	pub lineno: u32,
	pub column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
					let exception = Object::from(exception.to_object());
					Exception::clear(cx);

					let message = exception.get_as::<String>(cx, "message", ()).unwrap();
					let filename = exception.get_as::<String>(cx, "fileName", ()).unwrap();
					let lineno = exception.get_as::<u32>(cx, "lineNumber", ConversionBehavior::Clamp).unwrap();
					let column = exception.get_as::<u32>(cx, "columnNumber", ConversionBehavior::Clamp).unwrap();

					Some(Exception { message, file: filename, lineno, column })
				} else {
					None
				}
			} else {
				None
			}
		}
	}

	/// Clears all exceptions within the runtime.
	pub fn clear(cx: Context) {
		unsafe { JS_ClearPendingException(cx) };
	}

	/// Formats the exception as an error message.
	pub fn format(&self) -> String {
		if !self.file.is_empty() && self.lineno != 0 && self.column != 0 {
			format!("Uncaught exception at {}:{}:{} - {}", self.file, self.lineno, self.column, self.message)
		} else {
			format!("Uncaught exception - {}", self.message)
		}
	}
}

impl StackRecord {
	#[cfg(feature = "sourcemap")]
	pub fn transform_with_sourcemap(&mut self, sourcemap: &SourceMap) {
		if let Some(token) = sourcemap.lookup_token(self.lineno - 1, self.column - 1) {
			self.lineno = token.get_src_line() + 1;
			self.column = token.get_src_col() + 1;
		}
	}
}

impl ErrorReport {
	/// Creates a new [ErrorReport] with the given [Exception] and no stack.
	pub fn new(exception: Exception) -> ErrorReport {
		ErrorReport { exception, stack: None }
	}

	/// Creates a new [ErrorReport] with the given [Exception] and the current stack.
	pub fn new_with_stack(cx: Context, exception: Exception) -> ErrorReport {
		let stack = unsafe {
			capture_stack!(in(cx) let stack);
			let stack = stack;
			stack.map(|s| parse_stack(&s.as_string(None, StackFormat::SpiderMonkey).unwrap()))
		};
		ErrorReport { exception, stack }
	}

	#[cfg(feature = "sourcemap")]
	pub fn transform_with_sourcemap(&mut self, sourcemap: &SourceMap) {
		if let Some(token) = sourcemap.lookup_token(self.exception.lineno - 1, self.exception.column - 1) {
			self.exception.lineno = token.get_src_line() + 1;
			self.exception.column = token.get_src_col() + 1;
		}
		if let Some(ref mut stack) = self.stack {
			for record in stack {
				record.transform_with_sourcemap(sourcemap);
			}
		}
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

impl Display for ErrorReport {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.exception.format())?;
		if let Some(ref stack) = self.stack {
			f.write_str(NEWLINE)?;
			f.write_str(&format_stack(stack))?;
		}
		Ok(())
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
		string.push_str(&record.to_string());
		string.push_str(NEWLINE);
	}
	string
}
