/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate ion;
#[macro_use]
extern crate mozjs;

use ion::IonContext;
use ion::objects::object::IonObject;
use runtime::StandardModules;

pub mod assert;
mod fs;
mod path;
mod url;

#[derive(Default)]
pub struct Modules;

impl StandardModules for Modules {
	fn init(cx: IonContext, global: IonObject) -> bool {
		unsafe { assert::init(cx, global) && fs::init(cx, global) && path::init(cx, global) && url::init(cx, global) }
	}
}
