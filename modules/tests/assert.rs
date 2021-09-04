/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;

use mozjs::rust::Runtime;

use modules::init_modules;
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::globals::{init_globals, new_global};
use runtime::modules::{init_module_loaders, IonModule};
use runtime::new_runtime;

#[test]
fn assert() {
	CONFIG
		.set(Config::default().log_level(LogLevel::Debug))
		.expect("Config Initialisation Failed");

	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_module_loaders(rt.cx());
	init_globals(rt.cx(), global);
	init_modules(rt.cx(), global);

	assert!(
		eval_module(&rt, Path::new("./tests/scripts/assert/success/assert.js")).is_ok(),
		"Exception was thrown in: success/assert.js"
	);
	assert!(
		eval_module(&rt, Path::new("./tests/scripts/assert/success/debug-assert.js")).is_ok(),
		"Exception was thrown in: success/debug-assert.js"
	);
	assert!(
		eval_module(&rt, Path::new("./tests/scripts/assert/failure/assert.js")).is_err(),
		"No exception was thrown in: failure/assert.js"
	);
	assert!(
		eval_module(&rt, Path::new("./tests/scripts/assert/failure/debug-assert.js")).is_err(),
		"No exception was thrown in: failure/debug-assert.js"
	);
}

pub fn eval_module(rt: &Runtime, path: &Path) -> Result<(), ()> {
	let script = read_to_string(path).unwrap();
	if let Err(e) = IonModule::compile(rt.cx(), path.file_name().unwrap().to_str().unwrap(), Some(path), &script) {
		e.print();
		Err(())
	} else {
		Ok(())
	}
}
