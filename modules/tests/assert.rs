/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use mozjs::rust::JSEngine;

use modules::Assert;
use runtime::{Runtime, RuntimeBuilder};
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::modules::Module;

const OK: (&str, &str) = ("ok", include_str!("scripts/assert/ok.js"));
const EQUALS: (&str, &str) = ("equals", include_str!("scripts/assert/equals.js"));
const THROWS: (&str, &str) = ("throws", include_str!("scripts/assert/throws.js"));
const FAIL: (&str, &str) = ("fail", include_str!("scripts/assert/fail.js"));

#[test]
fn assert() {
	CONFIG.set(Config::default().log_level(LogLevel::Debug)).unwrap();
	let engine = JSEngine::init().unwrap();
	let rt = RuntimeBuilder::<Assert>::new().modules().standard_modules().build(engine.handle());

	eval_module(&rt, OK);
	eval_module(&rt, EQUALS);
	eval_module(&rt, THROWS);
	eval_module(&rt, FAIL);
}

pub fn eval_module(rt: &Runtime, test: (&str, &str)) {
	let (test, script) = test;
	let filename = format!("{}.js", test);
	let path = format!("./tests/scripts/assert/{}.js", test);
	let error = format!("Assertion Failed: assert.{}", test);

	let module = Module::compile(rt.cx(), &filename, Some(Path::new(&path)), script);
	assert!(module.is_err(), "No exception was thrown in: {}", filename);
	let report = module.unwrap_err();
	assert_eq!(report.inner().exception.message, error, "{}: {}", filename, report.inner());
}
