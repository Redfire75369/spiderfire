/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate mozjs;

use mozjs::jsapi::*;

mod fs;

pub fn init_modules(cx: *mut JSContext, global: *mut JSObject) -> bool {
	fs::fs::init_fs(cx, global)
}
