/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::cell::RefCell;
use ::std::collections::hash_map::{Entry, HashMap};
use ::std::ffi::CString;
use ::std::ptr;

use chrono::{DateTime, offset::Utc};
use mozjs::jsapi::*;
use mozjs::jsval::ObjectValue;

use ion::functions::arguments::Arguments;
use ion::functions::macros::{IonContext, IonResult};
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

#[apply(js_fn!)]
fn log(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Info {
		print_indent(false);
		print_args(cx, values, false);
		println!();
	}

	Ok(())
}

#[apply(js_fn!)]
fn warn(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Warn {
		print_indent(true);
		print_args(cx, values, true);
		println!();
	}

	Ok(())
}

#[apply(js_fn!)]
fn error(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Error {
		print_indent(true);
		print_args(cx, values, true);
		println!();
	}

	Ok(())
}

#[apply(js_fn!)]
fn debug(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level == LogLevel::Debug {
		print_indent(false);
		print_args(cx, values, false);
		println!();
	}

	Ok(())
}

#[apply(js_fn!)]
fn assert(cx: IonContext, assertion: bool, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Error {
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
	}

	Ok(())
}

#[apply(js_fn!)]
fn clear() -> IonResult<()> {
	INDENTS.with(|indents| {
		*indents.borrow_mut() = 0;
	});

	println!("{}", ANSI_CLEAR);
	println!("{}", ANSI_CLEAR_SCREEN_DOWN);

	Ok(())
}

#[apply(js_fn!)]
fn trace(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
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

#[apply(js_fn!)]
fn group(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
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

#[apply(js_fn!)]
fn group_end() -> IonResult<()> {
	INDENTS.with(|indents| {
		let mut indents = indents.borrow_mut();
		*indents = (*indents).max(1) - 1;
	});

	Ok(())
}

#[apply(js_fn!)]
fn count(label: String) -> IonResult<()> {
	let label = if label.as_str() == "undefined" {
		String::from(DEFAULT_LABEL)
	} else {
		label
	};

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

#[apply(js_fn!)]
fn count_reset(label: String) -> IonResult<()> {
	let label = if label.as_str() == "undefined" {
		String::from(DEFAULT_LABEL)
	} else {
		label
	};

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

#[apply(js_fn!)]
fn time(label: String) -> IonResult<()> {
	let label = if label.as_str() == "undefined" {
		String::from(DEFAULT_LABEL)
	} else {
		label
	};

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

#[apply(js_fn!)]
fn time_log(cx: IonContext, label: String, #[varargs] values: Vec<Value>) -> IonResult<()> {
	let label = if label.as_str() == "undefined" {
		String::from(DEFAULT_LABEL)
	} else {
		label
	};

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
					print!("{}: {}ms", label, duration);
					print_args(cx, values, false);
					println!();
				}
			}
		}
	});

	Ok(())
}

#[apply(js_fn!)]
fn time_end(cx: IonContext, label: String, #[varargs] values: Vec<Value>) -> IonResult<()> {
	let label = if label.as_str() == "undefined" {
		String::from(DEFAULT_LABEL)
	} else {
		label
	};

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
					print!("{}: {}ms", label, duration);
					print_args(cx, values, false);
					println!();
				}
			}
		}
	});

	Ok(())
}

// TODO: Create console.table
const METHODS: &[JSFunctionSpecWithHelp] = &[
	JSFunctionSpecWithHelp {
		name: "log\0".as_ptr() as *const i8,
		call: Some(log),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "log([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "info\0".as_ptr() as *const i8,
		call: Some(log),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "info([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "dir\0".as_ptr() as *const i8,
		call: Some(log),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "dir([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "dirxml\0".as_ptr() as *const i8,
		call: Some(log),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "dirxml([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "warn\0".as_ptr() as *const i8,
		call: Some(warn),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "warn([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "error\0".as_ptr() as *const i8,
		call: Some(error),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "error([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "debug\0".as_ptr() as *const i8,
		call: Some(debug),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "debug([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "assert\0".as_ptr() as *const i8,
		call: Some(assert),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "assert(condition, [args ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "clear\0".as_ptr() as *const i8,
		call: Some(clear),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "clear()\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "trace\0".as_ptr() as *const i8,
		call: Some(trace),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "trace([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "group\0".as_ptr() as *const i8,
		call: Some(group),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "group([label])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "groupCollapsed\0".as_ptr() as *const i8,
		call: Some(group),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "groupCollapsed([label])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "groupEnd\0".as_ptr() as *const i8,
		call: Some(group_end),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "groupEnd()\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "count\0".as_ptr() as *const i8,
		call: Some(count),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "count(label)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "countReset\0".as_ptr() as *const i8,
		call: Some(count_reset),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "countReset(label)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "time\0".as_ptr() as *const i8,
		call: Some(time),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "time(label)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "timeLog\0".as_ptr() as *const i8,
		call: Some(time_log),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "timeLog(label[, exp])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "timeEnd\0".as_ptr() as *const i8,
		call: Some(time_end),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "timeEnd(label)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: ptr::null_mut(),
		call: None,
		nargs: 0,
		flags: 0,
		jitInfo: ptr::null_mut(),
		usage: ptr::null_mut(),
		help: ptr::null_mut(),
	},
];

pub fn define(cx: *mut JSContext, global: *mut JSObject) -> bool {
	unsafe {
		rooted!(in(cx) let obj = JS_NewPlainObject(cx));
		rooted!(in(cx) let obj_val = ObjectValue(obj.get()));
		rooted!(in(cx) let rglobal = global);
		return JS_DefineFunctionsWithHelp(cx, obj.handle().into(), METHODS.as_ptr())
			&& JS_DefineProperty(cx, rglobal.handle().into(), "console\0".as_ptr() as *const i8, obj_val.handle().into(), 0);
	}
}
