/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::io::{stdin, stdout, Write};
use ::std::process;
use ::std::ptr;

use mozjs::jsapi::*;
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};

use ion::objects::object::IonObject;
use runtime::init;

use crate::evaluate::eval_inline;

pub fn start_repl() {
	let engine = JSEngine::init().expect("JS Engine Initialisation Failed");
	let rt = Runtime::new(engine.handle());

	assert!(!rt.cx().is_null(), "JSContext Creation Failed");

	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(rt.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _ac = JSAutoRealm::new(rt.cx(), global);

	init(rt.cx(), unsafe { IonObject::from(global) });

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

			input = input + "\n" + &line;
			if multiline.0 <= 0 && multiline.1 <= 0 && multiline.2 <= 0 {
				break;
			}
		}

		if input.len() != 1 {
			eval_inline(&rt, global, &input);
		} else if input == "exit\n" {
			process::exit(0);
		}
	}
}
