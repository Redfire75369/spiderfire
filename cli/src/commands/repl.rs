/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use rustyline::{Config, Editor};
use rustyline::config::Builder;
use rustyline::error::ReadlineError;

use runtime::globals::{init_globals, new_global};
use runtime::microtask_queue::init_microtask_queue;
use runtime::new_runtime;

use crate::evaluate::eval_inline;

pub fn start_repl() {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_globals(rt.cx(), global);
	init_microtask_queue(rt.cx());

	let mut repl = Editor::<()>::with_config(rustyline_config());
	let mut terminate: u8 = 0;

	loop {
		let mut input = String::new();
		let mut lines = 0;
		let mut multiline: (u16, u16, u16) = (0, 0, 0); // (), [], {}
		loop {
			let mut line = String::new();

			if lines == 0 {
				match repl.readline("> ") {
					Ok(input) => line = input,
					Err(error) => terminate += handle_error(error),
				}
			} else {
				match repl.readline("") {
					Ok(input) => line = input,
					Err(error) => terminate += handle_error(error),
				}
			}

			if terminate == 1 {
				println!("Press Ctrl+C to exit again.");
				break;
			} else if terminate > 1 {
				break;
			}

			let mut chars = line.chars();
			while let Some(ch) = chars.next() {
				match ch {
					'(' => multiline.0 += 1,
					')' => multiline.0 -= 1,
					'[' => multiline.1 += 1,
					']' => multiline.1 -= 1,
					'{' => multiline.2 += 1,
					'}' => multiline.2 -= 1,
					_ => (),
				}
			}

			input = (input + "\n" + &line.trim()).trim().to_owned();
			lines += 1;
			if multiline.0 <= 0 && multiline.1 <= 0 && multiline.2 <= 0 {
				break;
			}
		}

		if input == "exit" || terminate > 1 {
			break;
		}

		if !input.is_empty() {
			terminate = 0;
			eval_inline(&rt, &input);
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

fn rustyline_config() -> Config {
	let builder = Builder::new();
	builder.tab_stop(4).build()
}
