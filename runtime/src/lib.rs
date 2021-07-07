/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate ion;
#[macro_use]
extern crate mozjs;

use ion::functions::macros::IonContext;
use ion::objects::object::IonObject;

use crate::globals::console;

pub mod config;
pub mod globals;
pub mod modules;

pub fn init(cx: IonContext, global: IonObject) -> bool {
	console::define(cx, global)
}
