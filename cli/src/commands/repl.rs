/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::rust::{JSEngine, Runtime};
use rustyline::Editor;
use rustyline::error::ReadlineError;

use ion::Context;
use modules::Modules;
use runtime::RuntimeBuilder;

use crate::evaluate::eval_inline;
use crate::repl::{ReplHelper, rustyline_config};

pub(crate) async fn start_repl() {
	let engine = JSEngine::init().unwrap();
	let rt = Runtime::new(engine.handle());

	let cx = &mut Context::from_runtime(&rt);
	let rt = RuntimeBuilder::<(), _>::new()
		.microtask_queue()
		.macrotask_queue()
		.standard_modules(Modules)
		.build(cx);

	let mut repl = match Editor::with_config(rustyline_config()) {
		Ok(repl) => repl,
		Err(err) => {
			eprintln!("{}", err);
			return;
		}
	};
	repl.set_helper(Some(ReplHelper));
	let mut terminate: u8 = 0;

	loop {
		let mut input = String::new();

		match repl.readline("> ") {
			Ok(i) => input = String::from(i.trim()),
			Err(error) => terminate += handle_error(error),
		}

		repl.add_history_entry(&input).unwrap();

		if terminate == 1 && input.is_empty() {
			println!("Press Ctrl+C again or Ctrl+D to exit.");
			continue;
		} else if terminate > 1 {
			break;
		}

		if !input.is_empty() && input != "exit" {
			terminate = 0;
			eval_inline(&rt, &input).await;
		}

		if terminate > 1 || input == "exit" {
			break;
		}
	}
}

fn handle_error(error: ReadlineError) -> u8 {
	match error {
		ReadlineError::Interrupted => 1,
		ReadlineError::Eof => 2,
		_ => 0,
	}
}
