/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;

use mozjs::jsapi::{ModuleEvaluate, ModuleInstantiate};
use mozjs::jsval::UndefinedValue;
use mozjs::rust::Runtime;

use ion::exception::{ErrorReport, Exception};
use ion::objects::object::IonObject;
use ion::print::print_value;
use modules::init_modules;
use runtime::globals::{init_globals, new_global};
use runtime::modules::{compile_module, init_module_loaders};
use runtime::new_runtime;

pub fn eval_inline(rt: &Runtime, global: IonObject, source: &str) {
	let filename = "inline.js";
	let line_number = 1;

	rooted!(in(rt.cx()) let rooted_global = global.raw());
	rooted!(in(rt.cx()) let mut rval = UndefinedValue());

	let res = rt.evaluate_script(rooted_global.handle(), source, filename, line_number, rval.handle_mut());

	if res.is_ok() {
		print_value(rt.cx(), rval.get(), 0, false);
		println!();
	} else {
		let exception = unsafe { Exception::new(rt.cx()).unwrap() };
		ErrorReport::new(exception).print();
	}
}

pub fn eval_script(path: &Path) {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_globals(rt.cx(), global);

	let script = read_script(path).expect("");

	rooted!(in(rt.cx()) let rooted_global = global.raw());
	rooted!(in(rt.cx()) let mut rval = UndefinedValue());

	// TODO: Replace Runtime::evaluate_script with custom usage of Evaluate/Evaluate2
	let res = rt.evaluate_script(
		rooted_global.handle(),
		&script,
		path.file_name().unwrap().to_str().unwrap(),
		1,
		rval.handle_mut(),
	);

	if res.is_err() {
		let exception = unsafe { Exception::new(rt.cx()).unwrap() };
		ErrorReport::new_with_stack(rt.cx(), exception).print();
	}
}

pub fn eval_module(path: &Path) {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_module_loaders(rt.cx());
	init_globals(rt.cx(), global);
	init_modules(rt.cx(), global);

	let script = read_script(path).expect("");

	rooted!(in(rt.cx()) let module = unsafe {
		compile_module(rt.cx(), &String::from(path.file_name().unwrap().to_str().unwrap()), Some(path), &script).unwrap()
	});

	unsafe {
		if ModuleInstantiate(rt.cx(), module.handle().into()) {
			rooted!(in(rt.cx()) let mut rval = UndefinedValue());
			if !ModuleEvaluate(rt.cx(), module.handle().into(), rval.handle_mut().into()) {
				let exception = Exception::new(rt.cx()).unwrap();
				ErrorReport::new_with_stack(rt.cx(), exception).print();
			}
		} else {
			let exception = Exception::new(rt.cx()).unwrap();
			ErrorReport::new_with_stack(rt.cx(), exception).print();
		}
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
