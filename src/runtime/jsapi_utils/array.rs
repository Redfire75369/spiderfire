/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::*;

pub fn is_array(cx: *mut JSContext, val: Value) -> bool {
	let mut bool = false;
	if val.is_object() {
		unsafe {
			rooted!(in(cx) let rval = val.to_object());
			IsArray(cx, rval.handle().into(), &mut bool);
		}
	}

	bool
}
