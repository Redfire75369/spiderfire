/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;

use mozjs::jsval::UndefinedValue;
use mozjs::rooted;

use ion::exception::{ErrorReport, Exception};
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::globals::{init_globals, new_global};
use runtime::new_runtime;

#[test]
fn console() {
	CONFIG
		.set(Config::default().log_level(LogLevel::Debug).script(true))
		.expect("Config Initialisation Failed");
	assert!(eval_script(Path::new("./tests/scripts/console.js")).is_ok());
}

pub fn eval_script(path: &Path) -> Result<(), ()> {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_globals(rt.cx(), global);

	let script = read_script(path).expect("");

	rooted!(in(rt.cx()) let rooted_global = global.raw());
	rooted!(in(rt.cx()) let mut rval = UndefinedValue());

	let res = rt.evaluate_script(
		rooted_global.handle(),
		&script,
		path.file_name().unwrap().to_str().unwrap(),
		1,
		rval.handle_mut(),
	);

	if res.is_err() {
		let exception = unsafe { Exception::new(rt.cx()).unwrap() };
		ErrorReport::new_with_stack(rt.cx(), exception).print();
	}

	return res;
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
