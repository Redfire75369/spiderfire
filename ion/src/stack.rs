/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{fmt, ptr};
use std::fmt::{Display, Formatter};

use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{BuildStackString, JSObject, JSString, StackFormat};
#[cfg(feature = "sourcemap")]
use sourcemap::SourceMap;

use crate::Context;
use crate::format::{INDENT, NEWLINE};
use crate::utils::normalise_path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
	pub file: String,
	pub lineno: u32,
	pub column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackRecord {
	pub function: Option<String>,
	pub location: Location,
}

#[derive(Clone, Debug)]
pub struct Stack {
	pub records: Vec<StackRecord>,
}

impl Location {
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

impl StackRecord {
	#[cfg(feature = "sourcemap")]
	pub fn transform_with_sourcemap(&mut self, sourcemap: &SourceMap) {
		self.location.transform_with_sourcemap(sourcemap);
	}
}

impl Display for StackRecord {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str(self.function.as_deref().unwrap_or(""))?;
		f.write_str("@")?;
		f.write_str(&self.location.file)?;
		f.write_str(":")?;
		f.write_str(&self.location.lineno.to_string())?;
		f.write_str(":")?;
		f.write_str(&self.location.column.to_string())?;
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
			let file = String::from(normalise_path(file).to_str().unwrap());
			let lineno = lineno.parse().unwrap();
			let column = column.parse().unwrap();

			records.push(StackRecord {
				function,
				location: Location { file, lineno, column },
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
