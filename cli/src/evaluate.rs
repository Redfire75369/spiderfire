/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::path::Path;

use mozjs::rust::Runtime;

use ion::format::config::Config;
use ion::format::format_value;
use ion::script::IonScript;
use modules::init_modules;
use runtime::globals::{init_globals, new_global};
use runtime::microtask_queue::{init_microtask_queue, MicrotaskQueue};
use runtime::modules::{init_module_loaders, IonModule};
use runtime::new_runtime;

pub fn eval_inline(rt: &Runtime, queue: &MicrotaskQueue, source: &str) {
	let result = IonScript::compile_and_evaluate(rt.cx(), "inline.js", source);

	if queue.run_jobs(rt.cx()).is_err() {
		eprintln!("Error occured while executing microtask.");
	}
	match result {
		Ok(v) => println!("{}", format_value(rt.cx(), Config::default().quoted(true), v)),
		Err(e) => e.print(),
	}
}

pub fn eval_script(path: &Path) {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_globals(rt.cx(), global);
	let queue = init_microtask_queue(rt.cx());

	let script = read_script(path).unwrap();
	let result = IonScript::compile_and_evaluate(rt.cx(), path.file_name().unwrap().to_str().unwrap(), &script);

	if queue.run_jobs(rt.cx()).is_err() {
		eprintln!("Error occured while executing microtask.");
	}
	match result {
		Ok(v) => println!("{}", format_value(rt.cx(), Config::default().quoted(true), v)),
		Err(e) => e.print(),
	}
}

pub fn eval_module(path: &Path) {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_globals(rt.cx(), global);
	let queue = init_microtask_queue(rt.cx());
	init_module_loaders(rt.cx());
	init_modules(rt.cx(), global);

	let script = read_script(path).unwrap();
	let result = IonModule::compile(rt.cx(), path.file_name().unwrap().to_str().unwrap(), Some(path), &script);

	if queue.run_jobs(rt.cx()).is_err() {
		eprintln!("Error occured while executing microtask.");
	}
	if let Err(e) = result {
		e.print();
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
