/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::io::{stdin, stdout, Write};
use std::process;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

use ctrlc::set_handler;

use runtime::globals::{init_globals, new_global};
use runtime::microtask_queue::init_microtask_queue;
use runtime::new_runtime;

use crate::evaluate::eval_inline;

pub fn start_repl() {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_globals(rt.cx(), global);
	init_microtask_queue(rt.cx());

	let terminate = Arc::new(AtomicBool::new(false));
	let t = terminate.clone();

	let handler = set_handler(move || {
		if t.load(SeqCst) {
			process::exit(0);
		}
		t.store(true, SeqCst);
		println!();
		println!("Press Ctrl+C again to exit.");
	});

	if handler.is_err() {
		println!("Failed to initialise termination handler.")
	}

	loop {
		print!("> ");
		stdout().flush().expect("Failed to flush stdout :(");

		let mut input = String::new();
		let mut multiline: (i8, i8, i8) = (0, 0, 0); // (), [], {}
		loop {
			let mut line = String::new();
			stdin().read_line(&mut line).expect("Failed to read input :(");

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
			if multiline.0 <= 0 && multiline.1 <= 0 && multiline.2 <= 0 {
				break;
			}
		}

		terminate.store(true, SeqCst);

		if input == "exit" {
			process::exit(0);
		}

		if !input.is_empty() {
			eval_inline(&rt, &input);
		}
	}
}
