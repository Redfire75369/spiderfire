/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use mozjs::rust::{JSEngine, Runtime};

use ion::Context;
use ion::script::Script;
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::RuntimeBuilder;

const FILE_NAME: &str = "console.js";
const SCRIPT: &str = include_str!("scripts/console.js");

#[test]
fn console() {
	CONFIG.set(Config::default().log_level(LogLevel::Debug).script(true)).unwrap();

	let engine = JSEngine::init().unwrap();
	let rt = Runtime::new(engine.handle());

	let cx = &mut Context::from_runtime(&rt);
	let rt = RuntimeBuilder::<()>::new().build(cx);

	let result = Script::compile_and_evaluate(rt.cx(), Path::new(FILE_NAME), SCRIPT);
	assert!(result.is_ok(), "Error: {:?}", result.unwrap_err());
}
