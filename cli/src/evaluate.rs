/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::fs::read_to_string;
use ::std::path::Path;
use ::std::ptr;

use mozjs::jsapi::*;
use mozjs::jsval::UndefinedValue;
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};

use ion::exceptions::exception::report_and_clear_exception;
use ion::print::println_value;
use modules::init_modules;
use runtime::init;
use runtime::modules::compile_module;

pub fn eval_inline(rt: &Runtime, global: *mut JSObject, source: &str) {
	let filename: &'static str = "inline.js";
	let line_number: u32 = 1;

	rooted!(in(rt.cx()) let rooted_global = global);
	rooted!(in(rt.cx()) let mut rval = UndefinedValue());

	let res = rt.evaluate_script(rooted_global.handle(), source, filename, line_number, rval.handle_mut());

	if res.is_ok() {
		println_value(rt.cx(), rval.get(), 0, false);
	} else {
		println!("Script Execution Failed :(");
		unsafe {
			report_and_clear_exception(rt.cx());
		}
	}
}

pub fn eval_script(path: &Path) {
	let engine = JSEngine::init().expect("JS Engine Initialisation Failed");
	let rt = Runtime::new(engine.handle());

	assert!(!rt.cx().is_null(), "JSContext Creation Failed");

	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(rt.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _ac = JSAutoRealm::new(rt.cx(), global);

	init(rt.cx(), global);

	if !path.is_file() {
		eprintln!("File not found: {}", path.display());
	}
	let script = read_to_string(path).unwrap_or_else(|_| String::from(""));
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
}

pub fn eval_module(path: &Path) {
	let engine = JSEngine::init().expect("JS Engine Initialisation Failed");
	let rt = Runtime::new(engine.handle());

	assert!(!rt.cx().is_null(), "JSContext Creation Failed");

	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(rt.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _ac = JSAutoRealm::new(rt.cx(), global);

	unsafe {
		SetModuleResolveHook(JS_GetRuntime(rt.cx()), Some(runtime::modules::resolve_module));
	}
	init(rt.cx(), global);
	init_modules(rt.cx(), global);

	if !path.is_file() {
		eprintln!("File not found: {}", path.display());
	}
	let script = read_to_string(path).unwrap_or_else(|_| String::from(""));

	rooted!(in(rt.cx()) let rooted_global = global);
	rooted!(in(rt.cx()) let module = unsafe { compile_module(rt.cx(), &String::from(path.file_name().unwrap().to_str().unwrap()), Some(path), &script).unwrap() });

	unsafe {
		if ModuleInstantiate(rt.cx(), module.handle().into()) {
			rooted!(in(rt.cx()) let mut rval = UndefinedValue());
			if !ModuleEvaluate(rt.cx(), module.handle().into(), rval.handle_mut().into()) {
				report_and_clear_exception(rt.cx());
			}
		} else {
			report_and_clear_exception(rt.cx());
		}
	}
}
