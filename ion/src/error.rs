/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::error::{throw_internal_error, throw_range_error, throw_type_error};
use mozjs::jsapi::JS_ReportErrorUTF8;

use crate::IonContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IonError {
	InternalError(String),
	RangeError(String),
	TypeError(String),
	Error(String),
	None,
}

impl IonError {
	pub fn throw(self, cx: IonContext) {
		unsafe {
			match self {
				IonError::InternalError(str) => throw_internal_error(cx, &str),
				IonError::RangeError(str) => throw_range_error(cx, &str),
				IonError::TypeError(str) => throw_type_error(cx, &str),
				IonError::Error(str) => JS_ReportErrorUTF8(cx, format!("{}\0", str).as_ptr() as *const i8),
				IonError::None => (),
			}
		}
	}
}
