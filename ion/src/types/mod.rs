/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsval::JSVal;

use crate::{Array, Context, Date};

pub mod values;

/// Checks if a [JSVal] is an array.
pub fn is_array(cx: Context, value: JSVal) -> bool {
	value.is_object() && Array::is_array_raw(cx, value.to_object())
}

/// Checks if a [JSVal] is a date.
pub fn is_date(cx: Context, value: JSVal) -> bool {
	value.is_object() && Date::is_date_raw(cx, value.to_object())
}
