/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::{Context, Object};

pub mod console;
pub mod microtasks;
pub mod timers;

pub fn init_globals(cx: Context, global: Object) -> bool {
	console::define(cx, global)
}

pub fn init_timers(cx: Context, global: Object) -> bool {
	timers::define(cx, global)
}

pub fn init_microtasks(cx: Context, global: Object) -> bool {
	microtasks::define(cx, global)
}
