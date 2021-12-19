/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::IonContext;
use ion::objects::object::IonObject;

pub mod console;
pub mod microtasks;
pub mod timers;

pub fn init_globals(cx: IonContext, global: IonObject) -> bool {
	unsafe { console::define(cx, global) }
}

pub fn init_timers(cx: IonContext, global: IonObject) -> bool {
	unsafe { timers::define(cx, global) }
}

pub fn init_microtasks(cx: IonContext, global: IonObject) -> bool {
	unsafe { microtasks::define(cx, global) }
}
