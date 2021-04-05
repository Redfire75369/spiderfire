/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::*;

use crate::runtime::globals::console;
use crate::modules::fs;

pub fn init(cx: *mut JSContext, global: *mut JSObject) -> bool {
	console::define(cx, global)
}

pub fn init_modules(cx: *mut JSContext, global: *mut JSObject) -> bool {
	fs::init_fs(cx, global)
}
