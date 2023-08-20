/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use mozjs::rust::{JSEngine, Runtime};

use ion::Context;
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::RuntimeBuilder;
use runtime::script::Script;

const FILE_NAME: &str = "console.js";
const SCRIPT: &str = include_str!("scripts/console.js");

#[test]
fn console() {
	CONFIG.set(Config::default().log_level(LogLevel::Debug).script(true)).unwrap();

	let engine = JSEngine::init().unwrap();
	let rt = Runtime::new(engine.handle());

	let cx = Context::new(rt.cx()).unwrap();
	let _rt = RuntimeBuilder::<()>::new().build(&cx);

	let result = Script::compile_and_evaluate(&cx, Path::new(FILE_NAME), SCRIPT);
	assert!(result.is_ok(), "Error: {:?}", result.unwrap_err());
}
