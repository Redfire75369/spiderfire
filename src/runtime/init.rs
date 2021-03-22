/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::*;

use crate::runtime::globals::console;
use crate::modules::fs;

pub(crate) fn init(cx: *mut JSContext, global: *mut JSObject) -> bool {
	return console::define(cx, global);
}

pub(crate) fn init_modules(cx: *mut JSContext, global: *mut JSObject) -> bool {
	return fs::init_fs(cx, global);
}
