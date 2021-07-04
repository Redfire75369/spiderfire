/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;
use std::ptr;

use mozjs::jsapi::{JS_GetRuntime, JS_NewGlobalObject, JSAutoRealm, ModuleEvaluate, ModuleInstantiate, OnNewGlobalHookOption, SetModuleResolveHook};
use mozjs::jsval::UndefinedValue;
use mozjs::rooted;
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};

use ion::exceptions::exception::report_and_clear_exception;
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::init;
use runtime::modules::{compile_module, resolve_module};

pub fn eval_module(path: &Path) -> Result<(), ()> {
	let engine = JSEngine::init().expect("JS Engine Initialisation Failed");
	let rt = Runtime::new(engine.handle());

	assert!(!rt.cx().is_null(), "JSContext Creation Failed");

	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(rt.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _ac = JSAutoRealm::new(rt.cx(), global);

	unsafe {
		SetModuleResolveHook(JS_GetRuntime(rt.cx()), Some(resolve_module));
	}
	init(rt.cx(), global);

	if !path.is_file() {
		panic!("File not found: {}", path.display());
	}
	let script = read_to_string(path).unwrap();

	rooted!(in(rt.cx()) let rooted_global = global);
	rooted!(in(rt.cx()) let module = unsafe { compile_module(rt.cx(), &String::from(path.file_name().unwrap().to_str().unwrap()), Some(path), &script).unwrap() });

	unsafe {
		return if ModuleInstantiate(rt.cx(), module.handle().into()) {
			rooted!(in(rt.cx()) let mut rval = UndefinedValue());
			if !ModuleEvaluate(rt.cx(), module.handle().into(), rval.handle_mut().into()) {
				report_and_clear_exception(rt.cx());
				return Err(());
			}
			Ok(())
		} else {
			report_and_clear_exception(rt.cx());
			Err(())
		};
	}
}

#[test]
fn modules() {
	let config = Config::initialise(LogLevel::Debug, false).unwrap();
	CONFIG.set(config).unwrap();
	assert!(eval_module(Path::new("./tests/scripts/module-import.js")).is_ok());
}
