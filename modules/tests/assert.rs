/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;

use mozjs::rust::JSEngine;

use ion::IonContext;
use ion::objects::object::IonObject;
use modules::assert;
use runtime::{Runtime, RuntimeBuilder, StandardModules};
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::modules::IonModule;

const OK_MESSAGE: &str = "Assertion Failed: assert.ok";
const EQUALS_MESSAGE: &str = "Assertion Failed: assert.equals";
const THROWS_MESSAGE: &str = "Assertion Failed: assert.throws";
const FAIL_MESSAGE: &str = "Assertion Failed: assert.fail";

#[derive(Default)]
struct AssertModule;

impl StandardModules for AssertModule {
	fn init(cx: IonContext, global: IonObject) -> bool {
		unsafe { assert::init(cx, global) }
	}
}

#[test]
fn assert() {
	CONFIG.set(Config::default().log_level(LogLevel::Debug)).unwrap();
	let engine = JSEngine::init().unwrap();
	let rt = RuntimeBuilder::<AssertModule>::new().modules().standard_modules().build(engine.handle());

	eval_module(&rt, concat!("./tests/scripts/assert/", "ok.js"), OK_MESSAGE);
	eval_module(&rt, concat!("./tests/scripts/assert/", "equals.js"), EQUALS_MESSAGE);
	eval_module(&rt, concat!("./tests/scripts/assert/", "throws.js"), THROWS_MESSAGE);
	eval_module(&rt, concat!("./tests/scripts/assert/", "fail.js"), FAIL_MESSAGE);
}

pub fn eval_module(rt: &Runtime, path: &str, expected_message: &str) {
	let path = Path::new(path);
	let filename = path.file_name().unwrap().to_str().unwrap();
	let script = read_to_string(path).unwrap();

	let module = IonModule::compile(rt.cx(), filename, Some(path), &script);
	assert!(module.is_err(), "No exception was thrown in: {}", filename);
	let report = module.unwrap_err();
	assert_eq!(report.inner().exception.message, expected_message, "{}: {}", filename, report);
}
