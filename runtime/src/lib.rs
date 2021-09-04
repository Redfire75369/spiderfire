/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate ion;
#[macro_use]
extern crate mozjs;

use mozjs::rust::{JSEngine, Runtime};

pub mod config;
pub mod globals;
pub mod microtask_queue;
pub mod modules;

pub fn new_runtime() -> (JSEngine, Runtime) {
	let engine = JSEngine::init().expect("JS Engine Initialisation Failed");
	let runtime = Runtime::new(engine.handle());

	assert!(!runtime.cx().is_null(), "JSContext Creation Failed");

	(engine, runtime)
}
