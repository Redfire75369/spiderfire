/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;

use mozjs::rust::JSEngine;

use runtime::config::{Config, CONFIG, LogLevel};
use runtime::modules::IonModule;
use runtime::RuntimeBuilder;

#[test]
fn modules() {
	CONFIG.set(Config::default().log_level(LogLevel::Debug)).unwrap();
	assert!(eval_module(Path::new("./tests/scripts/module-import.js")).is_ok());
}

fn eval_module(path: &Path) -> Result<(), ()> {
	let engine = JSEngine::init().unwrap();
	let rt = RuntimeBuilder::<()>::new().modules().build(engine.handle());

	let script = read_script(path).expect("");
	if IonModule::compile(rt.cx(), path.file_name().unwrap().to_str().unwrap(), Some(path), &script).is_ok() {
		Ok(())
	} else {
		Err(())
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
