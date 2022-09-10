/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::error;
use std::fmt::{Display, Formatter};

use mozjs::error::{throw_internal_error, throw_range_error, throw_type_error};
use mozjs::jsapi::JS_ReportErrorUTF8;

use crate::Context;

/// Represents errors that can be thrown in the runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
	kind: ErrorKind,
	message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
	Internal,
	Range,
	Type,
	Other,
}

impl Error {
	pub fn new(message: &str) -> Error {
		Error {
			kind: ErrorKind::Other,
			message: String::from(message),
		}
	}

	pub fn new_internal(message: &str) -> Error {
		Error {
			kind: ErrorKind::Internal,
			message: String::from(message),
		}
	}

	pub fn new_range(message: &str) -> Error {
		Error {
			kind: ErrorKind::Range,
			message: String::from(message),
		}
	}

	pub fn new_type(message: &str) -> Error {
		Error {
			kind: ErrorKind::Type,
			message: String::from(message),
		}
	}

	/// Throws the [Error]
	pub fn throw(&self, cx: Context) {
		let msg = &self.message;
		unsafe {
			match self.kind {
				ErrorKind::Internal => throw_internal_error(cx, msg),
				ErrorKind::Range => throw_range_error(cx, msg),
				ErrorKind::Type => throw_type_error(cx, msg),
				ErrorKind::Other => JS_ReportErrorUTF8(cx, format!("{}\0", msg).as_ptr() as *const i8),
			}
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}

impl<E: error::Error> From<E> for Error {
	fn from(error: E) -> Error {
		Error::new(&error.to_string())
	}
}
