/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;

use mozjs::rust::Runtime;

use ion::script::IonScript;
use ion::types::string::to_string;
use modules::init_modules;
use runtime::globals::{init_globals, new_global};
use runtime::modules::{init_module_loaders, IonModule};
use runtime::new_runtime;

pub fn eval_inline(rt: &Runtime, source: &str) {
	match IonScript::compile_and_evaluate(rt.cx(), "inline.js", source) {
		Ok(v) => println!("{}", to_string(rt.cx(), v)),
		Err(e) => e.print(),
	}
}

pub fn eval_script(path: &Path) {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_globals(rt.cx(), global);

	let script = read_script(path).expect("");
	match IonScript::compile_and_evaluate(rt.cx(), &path.file_name().unwrap().to_str().unwrap(), &script) {
		Ok(v) => println!("{}", to_string(rt.cx(), v)),
		Err(e) => e.print(),
	}
}

pub fn eval_module(path: &Path) {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_module_loaders(rt.cx());
	init_globals(rt.cx(), global);
	init_modules(rt.cx(), global);

	let script = read_script(path).expect("");
	IonModule::compile(rt.cx(), path.file_name().unwrap().to_str().unwrap(), Some(path), &script);
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
