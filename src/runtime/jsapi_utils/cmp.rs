/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::*;
use mozjs::rust::RootedGuard;

fn value_eq(cx: *mut JSContext, rvalue: RootedGuard<'_, Value>, rother: RootedGuard<'_, Value>) -> bool {
	let value = rvalue.get();
	let other = rother.get();
	if value.is_number() && other.is_number() {
		return value.to_number() == other.to_number();
	} else if value.is_boolean() && other.is_boolean() {
		return value.to_boolean() == other.to_boolean();
	} else if value.is_string() && other.is_string() {
		unsafe {
			return jsstr_to_string(cx, value.to_string()) == jsstr_to_string(cx, other.to_string());
		}
	} else if value.is_null_or_undefined() && other.is_null_or_undefined() {
		return true;
	} else if value.is_object() && other.is_object() {
		rooted!(in(cx) let rvalue = value);
		rooted!(in(cx) let rother = other);
		unsafe {
			return jsstr_to_string(cx, JS_ValueToSource(cx, rvalue.handle().into())) == jsstr_to_string(cx, JS_ValueToSource(cx, rother.handle().into()));
		}
	}

	false
}

fn value_arr_eq(cx: *mut JSContext, values: &Box<[Value]>, others: &Box<[Value]>) -> bool {
	return {
		let mut eq = true;
		if values.len() == others.len() {
			for i in 0..values.len() {
				rooted!(in(cx) let value = values[i]);
				rooted!(in(cx) let other = others[i]);
				eq = eq && value_eq(cx, value, other);
			}
		} else {
			eq = false;
		}

		eq
	};
}
