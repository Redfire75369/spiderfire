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

mod assert;
mod fs;
mod path;

pub fn init_modules(cx: IonContext, global: IonObject) -> bool {
	unsafe { assert::init(cx, global) && fs::init(cx, global) && path::init(cx, global) }
}
