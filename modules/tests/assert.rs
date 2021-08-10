/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;

use mozjs::jsapi::{ModuleEvaluate, ModuleInstantiate};
use mozjs::jsval::UndefinedValue;
use mozjs::rooted;

use ion::exception::{ErrorReport, Exception};
use modules::init_modules;
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::globals::{init_globals, new_global};
use runtime::modules::{compile_module, init_module_loaders};
use runtime::new_runtime;

// TODO: Convert test to use #[should_panic]
// #[should_panic(expected = "Assertion Failed: Failing Assertion")]
#[test]
fn assert() {
	CONFIG
		.set(Config::default().log_level(LogLevel::Debug))
		.expect("Config Initialisation Failed");
	assert!(
		eval_module(Path::new("./tests/scripts/assert.js")).is_ok(),
		"Failed to evaluate module: assert.js"
	);
}

pub fn eval_module(path: &Path) -> Result<(), ()> {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_module_loaders(rt.cx());
	init_globals(rt.cx(), global);
	init_modules(rt.cx(), global);

	let script = read_script(path).expect("");

	rooted!(in(rt.cx()) let module = unsafe {
		compile_module(rt.cx(), &String::from(path.file_name().unwrap().to_str().unwrap()), Some(path), &script).unwrap()
	});

	unsafe {
		return if ModuleInstantiate(rt.cx(), module.handle().into()) {
			rooted!(in(rt.cx()) let mut rval = UndefinedValue());
			if !ModuleEvaluate(rt.cx(), module.handle().into(), rval.handle_mut().into()) {
				let exception = Exception::new(rt.cx()).unwrap();
				ErrorReport::new_with_stack(rt.cx(), exception).print();
				return Err(());
			}
			Ok(())
		} else {
			let exception = Exception::new(rt.cx()).unwrap();
			ErrorReport::new_with_stack(rt.cx(), exception).print();
			Err(())
		};
	}
}

fn read_script(path: &Path) -> Option<String> {
	if path.is_file() {
		if let Ok(script) = read_to_string(path) {
			Some(script)
		} else {
			eprintln!("Failed to read file: {}", path.display());
			None
		}
	} else {
		eprintln!("File not found: {}", path.display());
		None
	}
}
