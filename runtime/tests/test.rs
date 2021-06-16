/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::Path;
use std::thread;

use eval::*;
use runtime::config::{Config, CONFIG, LogLevel};

mod eval;

#[test]
fn console() {
	let config = Config::initialise(LogLevel::Debug, true).unwrap();
	CONFIG.set(config).unwrap();
	assert!(eval_script(Path::new("./scripts/console.js")).is_ok());
}

#[test]
fn module() {
	// [TODO]: Reset OnceCell and run test in new thread/process
	// let config = Config::initialise(LogLevel::Debug, false).unwrap();
	// CONFIG.set(config).unwrap();
	// assert!(eval_module(Path::new("./scripts/module.js")).is_ok());
}
