/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::error::{throw_internal_error, throw_range_error, throw_type_error};
use mozjs::jsapi::JS_ReportErrorUTF8;

use crate::Context;

/// Represents errors that can be thrown in the runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
	InternalError(String),
	RangeError(String),
	TypeError(String),
	Error(String),
	None,
}

impl Error {
	/// Throws the [Error]
	pub fn throw(self, cx: Context) {
		unsafe {
			match self {
				Error::InternalError(str) => throw_internal_error(cx, &str),
				Error::RangeError(str) => throw_range_error(cx, &str),
				Error::TypeError(str) => throw_type_error(cx, &str),
				Error::Error(str) => JS_ReportErrorUTF8(cx, format!("{}\0", str).as_ptr() as *const i8),
				Error::None => (),
			}
		}
	}
}
