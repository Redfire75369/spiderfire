/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{JS_ValueToSource, Value};

use crate::functions::macros::IonContext;

// TODO: Write function to convert objects to strings
/**
 * Converts a [Value] to a string.
 * Objects and functions are converted using [JS_ValueToSource]. This behaviour will be changed in the future.
 */
pub fn to_string(cx: IonContext, val: Value) -> String {
	rooted!(in(cx) let rval = val);

	if val.is_number() {
		val.to_number().to_string()
	} else if val.is_boolean() {
		val.to_boolean().to_string()
	} else if val.is_string() {
		unsafe { jsstr_to_string(cx, val.to_string()) }
	} else if val.is_object() {
		unsafe { jsstr_to_string(cx, JS_ValueToSource(cx, rval.handle().into())) }
	} else if val.is_null() {
		String::from("null")
	} else {
		String::from("undefined")
	}
}
