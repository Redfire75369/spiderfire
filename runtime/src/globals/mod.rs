/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use mozjs::jsapi::{JS_NewGlobalObject, JSAutoRealm, OnNewGlobalHookOption};
use mozjs::rust::{RealmOptions, SIMPLE_GLOBAL_CLASS};

use ion::IonContext;
use ion::objects::object::IonObject;

pub mod console;

pub fn new_global(cx: IonContext) -> (IonObject, JSAutoRealm) {
	unsafe {
		let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
		let c_options = RealmOptions::default();

		let global = JS_NewGlobalObject(cx, &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options);
		(IonObject::from(global), JSAutoRealm::new(cx, global))
	}
}

pub fn init_globals(cx: IonContext, global: IonObject) -> bool {
	unsafe { console::define(cx, global) }
}
