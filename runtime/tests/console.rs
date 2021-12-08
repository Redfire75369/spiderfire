/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;

use mozjs::jsapi::Value;
use mozjs::rust::JSEngine;

use ion::exception::ErrorReport;
use ion::format::format_value;
use ion::script::IonScript;
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::RuntimeBuilder;

#[test]
fn console() {
	CONFIG.set(Config::default().log_level(LogLevel::Debug).script(true)).unwrap();
	assert!(eval_script(Path::new("./tests/scripts/console.js")).is_ok());
}

fn eval_script(path: &Path) -> Result<Value, ErrorReport> {
	let engine = JSEngine::init().unwrap();
	let rt = RuntimeBuilder::<()>::new().build(engine.handle());

	let script = read_script(path).expect("");
	let res = IonScript::compile_and_evaluate(rt.cx(), "inline.js", &script);

	match res.clone() {
		Ok(v) => println!("{}", format_value(rt.cx(), ion::format::config::Config::default().quoted(true), v)),
		Err(e) => e.print(),
	}

	res
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
