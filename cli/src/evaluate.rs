/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::read_to_string;
use std::io::ErrorKind;
use std::path::Path;

use mozjs::rust::JSEngine;

use ion::format::config::FormatConfig;
use ion::format::format_value;
use ion::script::IonScript;
use modules::Modules;
use runtime::{Runtime, RuntimeBuilder};
use runtime::modules::IonModule;

pub fn eval_inline(rt: &Runtime, source: &str) {
	let result = IonScript::compile_and_evaluate(rt.cx(), "inline.js", source);

	match result {
		Ok(v) => println!("{}", format_value(rt.cx(), FormatConfig::default().quoted(true), v)),
		Err(report) => eprintln!("{}", report),
	}
	if rt.run_event_loop().is_err() {
		eprintln!("Unknown error occurred while executing microtask.");
	}
}

pub fn eval_script(path: &Path) {
	let engine = JSEngine::init().unwrap();
	let rt = RuntimeBuilder::<()>::new().macrotask_queue().microtask_queue().build(engine.handle());

	if let Some((script, filename)) = read_script(path) {
		let result = IonScript::compile_and_evaluate(rt.cx(), &filename, &script);

		match result {
			Ok(v) => println!("{}", format_value(rt.cx(), FormatConfig::default().quoted(true), v)),
			Err(report) => eprintln!("{}", report),
		}
		if rt.run_event_loop().is_err() {
			eprintln!("Unknown error occurred while executing microtask.");
		}
	}
}

pub fn eval_module(path: &Path) {
	let engine = JSEngine::init().unwrap();
	let rt = RuntimeBuilder::<Modules>::new()
		.macrotask_queue()
		.microtask_queue()
		.modules()
		.standard_modules()
		.build(engine.handle());

	if let Some((script, filename)) = read_script(path) {
		let result = IonModule::compile(rt.cx(), &filename, Some(path), &script);

		if let Err(report) = result {
			eprintln!("{}", report);
		}
		if rt.run_event_loop().is_err() {
			eprintln!("Unknown error occurred while executing microtask.");
		}
	}
}

fn read_script(path: &Path) -> Option<(String, String)> {
	match read_to_string(path) {
		Ok(script) => {
			let filename = String::from(path.file_name().unwrap().to_str().unwrap());
			Some((script, filename))
		}
		Err(error) => {
			eprintln!("Failed to read file: {}", path.display());
			match error.kind() {
				ErrorKind::NotFound => eprintln!("(File was not found)"),
				ErrorKind::PermissionDenied => eprintln!("Current User lacks permissions to read the file)"),
				_ => eprintln!("{:?}", error),
			}
			None
		}
	}
}
