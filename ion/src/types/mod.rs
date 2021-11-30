/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::Value;

use crate::IonContext;
use crate::objects::array::IonArray;
use crate::objects::date::IonDate;

pub mod values;

/// Checks if a [Value] is an array.
pub fn is_array(cx: IonContext, value: Value) -> bool {
	value.is_object() && unsafe { IonArray::is_array_raw(cx, value.to_object()) }
}

/// Checks if a [Value] is a date.
pub fn is_date(cx: IonContext, value: Value) -> bool {
	value.is_object() && unsafe { IonDate::is_date_raw(cx, value.to_object()) }
}
