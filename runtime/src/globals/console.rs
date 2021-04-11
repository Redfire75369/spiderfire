/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::cell::RefCell;
use ::std::collections::hash_map::{Entry, HashMap};
use ::std::ptr;

use chrono::{DateTime, offset::Utc};
use mozjs::jsapi::*;
use mozjs::jsval::{ObjectValue, UndefinedValue};

use ion::functions::arguments::Arguments;
use ion::print::{indent, INDENT, print_value};
use ion::types::string::to_string;

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

fn print_indent(is_error: bool) {
	INDENTS.with(|indents| {
		if !is_error {
			print!("{}", INDENT.repeat(*indents.borrow()));
		} else {
			eprint!("{}", INDENT.repeat(*indents.borrow()));
		}
	});
}

fn print_args(cx: *mut JSContext, args: Vec<Value>, is_error: bool) {
	for val in args.iter() {
		print_value(cx, *val, get_indents(), is_error);
		if !is_error {
			print!(" ");
		} else {
			eprint!(" ");
		}
	}
}

unsafe extern "C" fn log(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	print_indent(false);
	print_args(cx, args.range_full(), false);
	println!();

	true
}

// unsafe extern "C" fn debug(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
// 	let args = Arguments::new(argc, vp);
// 	args.rval().set(UndefinedValue());
//
// 	if Config::global().debug {
// 		log(cx, argc, vp)
// 	} else {
// 		true
// 	}
// }

unsafe extern "C" fn error(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	print_indent(true);
	print_args(cx, args.range_full(), true);
	println!();

	true
}

unsafe extern "C" fn assert(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);

	args.rval().set(UndefinedValue());
	if args.len() > 0 {
		if args.value(0).unwrap().is_boolean() {
			if args.value(0).unwrap().to_boolean() {
				return true;
			}
		} else {
			print_indent(true);
			eprintln!("Assertion Failed");
			return true;
		}

		if args.len() == 1 {
			print_indent(true);
			eprintln!("Assertion Failed");
			return true;
		}

		if args.value(1).unwrap().is_string() {
			print_indent(true);
			eprint!("Assertion Failed: {} ", to_string(cx, args.value(1).unwrap()));
			print_args(cx, args.range(2..), true);
			eprintln!();
			return true;
		}

		print_indent(true);
		eprint!("Assertion Failed: ");
		print_args(cx, args.range(1..), true);
		println!();
	} else {
		print_indent(true);
		eprintln!("Assertion Failed: ");
	}

	true
}

unsafe extern "C" fn clear(_cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	INDENTS.with(|indents| {
		*indents.borrow_mut() = 0;
	});

	println!("{}", ANSI_CLEAR);
	println!("{}", ANSI_CLEAR_SCREEN_DOWN);

	true
}

unsafe extern "C" fn trace(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	print_indent(false);
	print!("Trace: ");
	print_args(cx, args.range_full(), false);
	println!();

	capture_stack!(in(cx) let stack);
	println!(
		"{}",
		indent(
			&stack.unwrap().as_string(None, StackFormat::SpiderMonkey).unwrap(),
			get_indents() + 1,
			true
		)
	);

	true
}

unsafe extern "C" fn group(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	INDENTS.with(|indents| {
		let mut indents = indents.borrow_mut();
		print!("{}", "  ".repeat(*indents));
		print_args(cx, args.range_full(), false);
		println!();

		*indents = (*indents).min(usize::MAX - 1) + 1;
	});

	true
}

unsafe extern "C" fn group_end(_cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	INDENTS.with(|indents| {
		let mut indents = indents.borrow_mut();
		*indents = (*indents).max(1) - 1;
	});

	true
}

unsafe extern "C" fn count(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	let label = if args.len() > 0 {
		to_string(cx, args.values[0].get())
	} else {
		String::from(DEFAULT_LABEL)
	};

	print_indent(false);
	COUNT_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(v) => {
				println!("{}: {}", label, v.insert(1));
			}
			Entry::Occupied(mut o) => {
				println!("{}: {}", label, o.insert(o.get() + 1));
			}
		}
	});

	true
}

unsafe extern "C" fn count_reset(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	let label = if args.len() > 0 {
		to_string(cx, args.values[0].get())
	} else {
		String::from(DEFAULT_LABEL)
	};

	COUNT_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(_) => {
				print_indent(true);
				eprintln!("Count for {} does not exist", label);
			}
			Entry::Occupied(mut o) => {
				o.insert(0);
			}
		}
	});

	true
}

unsafe extern "C" fn time(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	let label = if args.len() > 0 {
		to_string(cx, args.values[0].get())
	} else {
		String::from(DEFAULT_LABEL)
	};

	TIMER_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(v) => {
				v.insert(Utc::now());
			}
			Entry::Occupied(_) => {
				print_indent(true);
				eprintln!("Timer {} already exists", label);
			}
		}
	});

	true
}

unsafe extern "C" fn time_log(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	let label = if args.len() > 0 {
		to_string(cx, args.values[0].get())
	} else {
		String::from(DEFAULT_LABEL)
	};

	TIMER_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(_) => {
				print_indent(true);
				eprintln!("Timer {} does not exist", label);
			}
			Entry::Occupied(o) => {
				let start_time = o.get();
				let duration = Utc::now().timestamp_millis() - start_time.timestamp_millis();
				print_indent(false);
				print!("{}: {}ms", label, duration);
				print_args(cx, args.range(1..), false);
				println!();
			}
		}
	});

	true
}

unsafe extern "C" fn time_end(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	let label = if args.len() > 0 {
		to_string(cx, args.values[0].get())
	} else {
		String::from(DEFAULT_LABEL)
	};

	TIMER_MAP.with(|map| {
		let mut map = map.borrow_mut();
		match (*map).entry(label.clone()) {
			Entry::Vacant(_) => {
				print_indent(true);
				eprintln!("Timer {} does not exist", label);
			}
			Entry::Occupied(o) => {
				let (_, start_time) = o.remove_entry();
				let duration = Utc::now().timestamp_millis() - start_time.timestamp_millis();
				print_indent(false);
				print!("{}: {}ms", label, duration);
				print_args(cx, args.range(1..), false);
				println!();
			}
		}
	});

	true
}

// TODO: console.table
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
	// JSFunctionSpecWithHelp {
	// 	name: "debug\0".as_ptr() as *const i8,
	// 	call: Some(debug),
	// 	nargs: 0,
	// 	flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
	// 	jitInfo: ptr::null_mut(),
	// 	usage: "debug([exp ...])\0".as_ptr() as *const i8,
	// 	help: "\0".as_ptr() as *const i8,
	// },
	JSFunctionSpecWithHelp {
		name: "warn\0".as_ptr() as *const i8,
		call: Some(error),
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
		usage: "trace([...args])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "group\0".as_ptr() as *const i8,
		call: Some(group),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "group([labels])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "groupCollapsed\0".as_ptr() as *const i8,
		call: Some(group),
		nargs: 0,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "groupCollapsed([labels])\0".as_ptr() as *const i8,
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
