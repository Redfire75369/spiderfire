/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::{Context, Object};

pub mod abort;
pub mod console;
pub mod microtasks;
pub mod timers;

pub fn init_globals<'cx>(cx: &'cx Context, global: &mut Object<'cx>) -> bool {
	console::define(cx, global)
}

pub fn init_timers<'cx>(cx: &'cx Context, global: &mut Object<'cx>) -> bool {
	timers::define(cx, global) && abort::define(cx, global)
}

pub fn init_microtasks<'cx>(cx: &'cx Context, global: &mut Object<'cx>) -> bool {
	microtasks::define(cx, global)
}
