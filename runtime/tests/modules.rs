/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use mozjs::rust::JSEngine;

use runtime::config::{Config, CONFIG, LogLevel};
use runtime::modules::IonModule;
use runtime::RuntimeBuilder;

const FILE_NAME: &str = "module-import.js";
const SCRIPT: &str = include_str!("scripts/module-import.js");

#[test]
fn modules() {
	CONFIG.set(Config::default().log_level(LogLevel::Debug)).unwrap();
	let engine = JSEngine::init().unwrap();
	let rt = RuntimeBuilder::<()>::new().modules().build(engine.handle());

	let path = format!("./tests/scripts/{}", FILE_NAME);
	assert!(IonModule::compile(rt.cx(), FILE_NAME, Some(Path::new(&path)), SCRIPT).is_ok())
}
