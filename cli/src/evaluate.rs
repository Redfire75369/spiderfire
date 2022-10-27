/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::OsStr;
use std::fs::read_to_string;
use std::io::ErrorKind;
use std::path::Path;

use mozjs::rust::JSEngine;
use mozjs::rust::Runtime as RustRuntime;
use sourcemap::SourceMap;

use ion::Context;
use ion::format::Config as FormatConfig;
use ion::format::format_value;
use modules::Modules;
use runtime::{Runtime, RuntimeBuilder};
use runtime::cache::locate_in_cache;
use runtime::cache::map::{save_sourcemap, transform_error_report_with_sourcemaps};
use runtime::config::Config;
use runtime::modules::handler::add_handler_reactions;
use runtime::modules::Module;
use runtime::script::Script;

pub async fn eval_inline(rt: &Runtime<'_, '_>, source: &str) {
	let result = Script::compile_and_evaluate(rt.cx(), Path::new("inline.js"), source);

	match result {
		Ok(v) => println!("{}", format_value(rt.cx(), FormatConfig::default().quoted(true), &v)),
		Err(report) => eprintln!("{}", report.format(rt.cx())),
	}
	run_event_loop(rt).await;
}

pub async fn eval_script(path: &Path) {
	let engine = JSEngine::init().unwrap();
	let rt = RustRuntime::new(engine.handle());
	let mut cx = rt.cx();

	let cx = Context::new(&mut cx);
	let rt = RuntimeBuilder::<Modules>::new()
		.microtask_queue()
		.macrotask_queue()
		.standard_modules()
		.build(&cx);

	if let Some((script, _)) = read_script(path) {
		let (script, sourcemap) = cache(path, script);
		if let Some(sourcemap) = sourcemap {
			save_sourcemap(&path, sourcemap);
		}
		let result = Script::compile_and_evaluate(rt.cx(), path, &script);

		match result {
			Ok(v) => println!("{}", format_value(rt.cx(), FormatConfig::default().quoted(true), &v)),
			Err(mut report) => {
				transform_error_report_with_sourcemaps(&mut report);
				eprintln!("{}", report.format(rt.cx()));
			}
		}
		run_event_loop(&rt).await;
	}
}

pub async fn eval_module(path: &Path) {
	let engine = JSEngine::init().unwrap();
	let rt = RustRuntime::new(engine.handle());
	let mut cx = rt.cx();

	let cx = Context::new(&mut cx);
	let rt = RuntimeBuilder::<Modules>::new()
		.microtask_queue()
		.macrotask_queue()
		.modules()
		.standard_modules()
		.build(&cx);

	if let Some((script, filename)) = read_script(path) {
		let (script, sourcemap) = cache(path, script);
		if let Some(sourcemap) = sourcemap {
			save_sourcemap(&path, sourcemap);
		}
		let result = Module::compile(rt.cx(), &filename, Some(path), &script);

		match result {
			Ok((_, Some(mut promise))) => {
				add_handler_reactions(rt.cx(), &mut promise);
			}
			Err(mut error) => {
				transform_error_report_with_sourcemaps(&mut error.report);
				eprintln!("{}", error.format(rt.cx()));
			}
			_ => {}
		}
		run_event_loop(&rt).await;
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

async fn run_event_loop(rt: &Runtime<'_, '_>) {
	if let Err(err) = rt.run_event_loop().await {
		if let Some(err) = err {
			eprintln!("{}", err.format(rt.cx()));
		} else {
			eprintln!("Unknown error occurred while executing microtask.");
		}
	}
}

fn cache(path: &Path, script: String) -> (String, Option<SourceMap>) {
	let is_typescript = Config::global().typescript && path.extension() == Some(OsStr::new("ts"));
	is_typescript
		.then(|| locate_in_cache(path, &script))
		.flatten()
		.map(|(s, sm)| (s, Some(sm)))
		.unwrap_or_else(|| (script, None))
}
