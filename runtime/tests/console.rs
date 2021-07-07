/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;
use std::ptr;

use mozjs::jsapi::{JS_NewGlobalObject, JSAutoRealm, OnNewGlobalHookOption};
use mozjs::jsval::UndefinedValue;
use mozjs::rooted;
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};

use ion::exceptions::exception::report_and_clear_exception;
use ion::objects::object::IonObject;
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::init;

pub fn eval_script(path: &Path) -> Result<(), ()> {
	let engine = JSEngine::init().expect("JS Engine Initialisation Failed");
	let rt = Runtime::new(engine.handle());

	assert!(!rt.cx().is_null(), "JSContext Creation Failed");

	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(rt.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _ac = JSAutoRealm::new(rt.cx(), global);

	init(rt.cx(), unsafe { IonObject::from(global) });

	if !path.is_file() {
		panic!("File not found: {}", path.display());
	}
	let script = read_to_string(path).unwrap();
	let line_number = 1;

	rooted!(in(rt.cx()) let rooted_global = global);
	rooted!(in(rt.cx()) let mut rval = UndefinedValue());

	let res = rt.evaluate_script(
		rooted_global.handle(),
		&script,
		path.file_name().unwrap().to_str().unwrap(),
		line_number,
		rval.handle_mut(),
	);

	if res.is_err() {
		unsafe {
			report_and_clear_exception(rt.cx());
		}
	}

	return res;
}

#[test]
fn console() {
	let config = Config::initialise(LogLevel::Debug, true).unwrap();
	CONFIG.set(config).unwrap();
	assert!(eval_script(Path::new("./tests/scripts/console.js")).is_ok());
}
