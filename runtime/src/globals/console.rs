/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::collections::hash_map::{Entry, HashMap};

use chrono::{DateTime, offset::Utc};
use mozjs::jsapi::{JS_DefineFunctions, JS_NewPlainObject, JSFunctionSpec, StackFormat, Value};
use mozjs::jsval::ObjectValue;

use ion::functions::arguments::Arguments;
use ion::functions::macros::{IonContext, IonResult};
use ion::objects::object::IonObject;
use ion::print::{indent, INDENT, print_value};
use ion::types::string::to_string;

use crate::config::{Config, LogLevel};

const ANSI_CLEAR: &str = "\x1b[1;1H";
const ANSI_CLEAR_SCREEN_DOWN: &str = "\x1b[0J";

const DEFAULT_LABEL: &str = "default";

thread_local! {
	static COUNT_MAP: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
	static TIMER_MAP: RefCell<HashMap<String, DateTime<Utc>>> = RefCell::new(HashMap::new());

	static INDENTS: RefCell<usize> = RefCell::new(0);
}

fn get_indents() -> usize {
	let mut ret: usize = 0;
	INDENTS.with(|indents| {
		ret = *indents.borrow();
	});
	ret
}

fn print_indent(is_stderr: bool) {
	INDENTS.with(|indents| {
		if !is_stderr {
			print!("{}", INDENT.repeat(*indents.borrow()));
		} else {
			eprint!("{}", INDENT.repeat(*indents.borrow()));
		}
	});
}

fn print_args(cx: IonContext, args: Vec<Value>, is_stderr: bool) {
	print_args_with_indents(cx, args, is_stderr, get_indents());
}

fn print_args_with_indents(cx: IonContext, args: Vec<Value>, is_stderr: bool, indents: usize) {
	for val in args.iter() {
		print_value(cx, *val, indents, is_stderr);
		if !is_stderr {
			print!(" ");
		} else {
			eprint!(" ");
		}
	}
}

// TODO: Convert to Undefinable<String> as null is a valid label
fn get_label(label: Option<String>) -> String {
	if let Some(label) = label {
		label
	} else {
		String::from(DEFAULT_LABEL)
	}
}

#[js_fn]
unsafe fn log(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Info {
		print_indent(false);
		print_args(cx, values, false);
		println!();
	}

	Ok(())
}

#[js_fn]
unsafe fn warn(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Warn {
		print_indent(true);
		print_args(cx, values, true);
		println!();
	}

	Ok(())
}

#[js_fn]
unsafe fn error(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Error {
		print_indent(true);
		print_args(cx, values, true);
		println!();
	}

	Ok(())
}

#[js_fn]
unsafe fn debug(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level == LogLevel::Debug {
		print_indent(false);
		print_args(cx, values, false);
		println!();
	}

	Ok(())
}

#[js_fn]
unsafe fn assert(cx: IonContext, assertion: Option<bool>, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Error {
		if let Some(assertion) = assertion {
			if assertion {
				return Ok(());
			}

			if values.len() == 0 {
				print_indent(true);
				eprintln!("Assertion Failed");
				return Ok(());
			}

			if values[0].is_string() {
				print_indent(true);
				eprint!("Assertion Failed: {} ", to_string(cx, values[0]));
				print_indent(true);
				print_args(cx, values[2..].to_vec(), true);
				eprintln!();
				return Ok(());
			}

			print_indent(true);
			eprint!("Assertion Failed: ");
			print_args(cx, values, true);
			println!();
		} else {
			eprintln!("Assertion Failed:");
			return Ok(());
		}
	}

	Ok(())
}

#[js_fn]
unsafe fn clear() -> IonResult<()> {
	INDENTS.with(|indents| {
		*indents.borrow_mut() = 0;
	});

	println!("{}", ANSI_CLEAR);
	println!("{}", ANSI_CLEAR_SCREEN_DOWN);

	Ok(())
}

#[js_fn]
unsafe fn trace(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level == LogLevel::Debug {
		print_indent(false);
		print!("Trace: ");
		print_args(cx, values, false);
		println!();

		capture_stack!(in(cx) let stack);
		println!(
			"{}",
			indent(
				&stack.unwrap().as_string(None, StackFormat::SpiderMonkey).unwrap(),
				get_indents() + 1,
				true,
			)
		);
	}

	Ok(())
}

#[js_fn]
unsafe fn group(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	INDENTS.with(|indents| {
		let mut indents = indents.borrow_mut();

		if Config::global().log_level >= LogLevel::Info {
			print!("{}", "  ".repeat(*indents));
			print_args_with_indents(cx, values, false, *indents);
			println!();
		}

		*indents = (*indents).min(usize::MAX - 1) + 1;
	});

	Ok(())
}

#[js_fn]
unsafe fn groupEnd() -> IonResult<()> {
	INDENTS.with(|indents| {
		let mut indents = indents.borrow_mut();
		*indents = (*indents).max(1) - 1;
	});

	Ok(())
}

#[js_fn]
unsafe fn count(label: Option<String>) -> IonResult<()> {
	let label = get_label(label);
	COUNT_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(v) => {
				let val = v.insert(1);
				if Config::global().log_level >= LogLevel::Info {
					print_indent(false);
					println!("{}: {}", label, val);
				}
			}
			Entry::Occupied(mut o) => {
				let val = o.insert(o.get() + 1);
				if Config::global().log_level >= LogLevel::Info {
					print_indent(false);
					println!("{}: {}", label, val);
				}
			}
		}
	});

	Ok(())
}

#[js_fn]
unsafe fn countReset(label: Option<String>) -> IonResult<()> {
	let label = get_label(label);
	COUNT_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(_) => {
				if Config::global().log_level >= LogLevel::Error {
					print_indent(true);
					eprintln!("Count for {} does not exist", label);
				}
			}
			Entry::Occupied(mut o) => {
				o.insert(0);
			}
		}
	});

	Ok(())
}

#[js_fn]
unsafe fn time(label: Option<String>) -> IonResult<()> {
	let label = get_label(label);
	TIMER_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(v) => {
				v.insert(Utc::now());
			}
			Entry::Occupied(_) => {
				if Config::global().log_level >= LogLevel::Error {
					print_indent(true);
					eprintln!("Timer {} already exists", label);
				}
			}
		}
	});

	Ok(())
}

#[js_fn]
unsafe fn timeLog(cx: IonContext, label: Option<String>, #[varargs] values: Vec<Value>) -> IonResult<()> {
	let label = get_label(label);
	TIMER_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(_) => {
				if Config::global().log_level >= LogLevel::Error {
					print_indent(true);
					eprintln!("Timer {} does not exist", label);
				}
			}
			Entry::Occupied(o) => {
				if Config::global().log_level >= LogLevel::Info {
					let start_time = o.get();
					let duration = Utc::now().timestamp_millis() - start_time.timestamp_millis();
					print_indent(false);
					print!("{}: {}ms ", label, duration);
					print_args(cx, values, false);
					println!();
				}
			}
		}
	});

	Ok(())
}

#[js_fn]
unsafe fn timeEnd(label: Option<String>) -> IonResult<()> {
	let label = get_label(label);
	TIMER_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(_) => {
				if Config::global().log_level >= LogLevel::Error {
					print_indent(true);
					eprintln!("Timer {} does not exist", label);
				}
			}
			Entry::Occupied(o) => {
				if Config::global().log_level >= LogLevel::Info {
					let (_, start_time) = o.remove_entry();
					let duration = Utc::now().timestamp_millis() - start_time.timestamp_millis();
					print_indent(false);
					print!("{}: {}ms - Timer Ended", label, duration);
					println!();
				}
			}
		}
	});

	Ok(())
}

// TODO: Create console.table
const METHODS: &[JSFunctionSpec] = &[
	function_spec!(log, 0),
	function_spec!(log, "info", 0),
	function_spec!(log, "dir", 0),
	function_spec!(log, "dirxml", 0),
	function_spec!(warn, 0),
	function_spec!(error, 0),
	function_spec!(debug, 0),
	function_spec!(assert, 0),
	function_spec!(clear, 0),
	function_spec!(trace, 0),
	function_spec!(group, 0),
	function_spec!(group, "groupCollapsed", 0),
	function_spec!(groupEnd, 0),
	function_spec!(count, 1),
	function_spec!(countReset, 1),
	function_spec!(time, 1),
	function_spec!(timeLog, 1),
	function_spec!(timeEnd, 1),
	JSFunctionSpec::ZERO,
];

pub fn define(cx: IonContext, mut global: IonObject) -> bool {
	unsafe {
		rooted!(in(cx) let console = JS_NewPlainObject(cx));
		return JS_DefineFunctions(cx, console.handle().into(), METHODS.as_ptr())
			&& global.define(cx, String::from("console"), ObjectValue(console.get()), 0);
	}
}
