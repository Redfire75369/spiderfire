/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::cell::RefCell;
use ::std::collections::hash_map::{Entry, HashMap};
use ::std::ptr;

use chrono::{DateTime, offset::Utc};
use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::*;
use mozjs::jsval::{ObjectValue, UndefinedValue};

use crate::config::Config;
use crate::runtime::jsapi_utils::print::print_value;

fn print_indent(is_error: bool) {
	INDENTS.with(|indents| {
		if !is_error {
			print!("{}", "  ".repeat(*indents.borrow_mut()));
		} else {
			eprint!("{}", "  ".repeat(*indents.borrow_mut()));
		}
	});
}

fn print_args(cx: *mut JSContext, args: &CallArgs, start: u32, is_error: bool) {
	for i in start..args.argc_ {
		rooted!(in(cx) let rval = args.get(i).get());
		print_value(cx, rval, is_error);
		print!(" ");
	}
}

thread_local! {
	static COUNT_MAP: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
	static TIMER_MAP: RefCell<HashMap<String, DateTime<Utc>>> = RefCell::new(HashMap::new());

	static INDENTS: RefCell<usize> = RefCell::new(0);
}

unsafe extern "C" fn log(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	print_indent(false);
	print_args(cx, &args, 0, false);
	println!();

	true
}

unsafe extern "C" fn debug(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	if Config::global().debug {
		log(cx, args.argc_, val)
	} else {
		true
	}
}

unsafe extern "C" fn error(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	print_indent(true);
	print_args(cx, &args, 0, true);
	println!();

	true
}

unsafe extern "C" fn assert(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	let rooted_condition = args.get(0);

	print_indent(true);

	if rooted_condition.get().is_boolean() {
		if rooted_condition.get().to_boolean() {
			return true;
		}
	} else {
		eprintln!("Assertion Failed");
		return true;
	}

	if args.argc_ == 1 {
		eprintln!("Assertion Failed");
		return true;
	}

	if args.get(1).get().is_string() {
		eprint!("Assertion Failed: {} ", jsstr_to_string(cx, args.get(1).get().to_string()));
		print_args(cx, &args, 2, true);
		eprintln!();
		return true;
	}

	eprint!("Assertion Failed: ");
	print_args(cx, &args, 1, true);
	println!();
	true
}

unsafe extern "C" fn group(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	INDENTS.with(|indents| {
		let mut indents = indents.borrow_mut();
		print!("{}", "  ".repeat(*indents));
		print_args(cx, &args, 0, false);
		println!();

		*indents = (*indents).min(usize::MAX - 1) + 1;
	});

	true
}

unsafe extern "C" fn group_end(_cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	INDENTS.with(|indents| {
		let mut indents = indents.borrow_mut();
		*indents = (*indents).max(1) - 1;
	});

	true
}

unsafe extern "C" fn count(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	let label = if args.argc_ > 0 && args.get(0).get().is_string() {
		jsstr_to_string(cx, args.get(0).get().to_string())
	} else {
		String::from("default")
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

unsafe extern "C" fn count_reset(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	let label = if args.argc_ > 0 && args.get(0).get().is_string() {
		jsstr_to_string(cx, args.get(0).get().to_string())
	} else {
		String::from("default")
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

unsafe extern "C" fn time(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	let label = if args.argc_ > 0 && args.get(0).get().is_string() {
		jsstr_to_string(cx, args.get(0).get().to_string())
	} else {
		String::from("default")
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

unsafe extern "C" fn time_log(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	let label = if args.argc_ > 0 && args.get(0).get().is_string() {
		jsstr_to_string(cx, args.get(0).get().to_string())
	} else {
		String::from("default")
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
				print_args(cx, &args, 1, false);
				println!();
			}
		}
	});

	return true;
}

unsafe extern "C" fn time_end(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);
	args.rval().set(UndefinedValue());

	let label = if args.argc_ > 0 && args.get(0).get().is_string() {
		jsstr_to_string(cx, args.get(0).get().to_string())
	} else {
		String::from("default")
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
				print_args(cx, &args, 1, false);
				println!();
			}
		}
	});

	true
}

// TODO: clear, table, trace
const METHODS: &'static [JSFunctionSpecWithHelp] = &[
	JSFunctionSpecWithHelp {
		name: "log\0".as_ptr() as *const i8,
		call: Some(log),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "log([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "info\0".as_ptr() as *const i8,
		call: Some(log),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "info([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "dir\0".as_ptr() as *const i8,
		call: Some(log),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "dir([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "dirxml\0".as_ptr() as *const i8,
		call: Some(log),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "dirxml([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "debug\0".as_ptr() as *const i8,
		call: Some(debug),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "debug([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "warn\0".as_ptr() as *const i8,
		call: Some(error),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "warn([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "error\0".as_ptr() as *const i8,
		call: Some(error),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "error([exp ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "assert\0".as_ptr() as *const i8,
		call: Some(assert),
		nargs: 1,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "assert(condition, [args ...])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "group\0".as_ptr() as *const i8,
		call: Some(group),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "group([labels])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "groupCollapsed\0".as_ptr() as *const i8,
		call: Some(group),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "groupCollapsed([labels])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "groupEnd\0".as_ptr() as *const i8,
		call: Some(group_end),
		nargs: 0,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "groupEnd()\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "count\0".as_ptr() as *const i8,
		call: Some(count),
		nargs: 1,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "count(label)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "countReset\0".as_ptr() as *const i8,
		call: Some(count_reset),
		nargs: 1,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "countReset(label)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "time\0".as_ptr() as *const i8,
		call: Some(time),
		nargs: 1,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "time(label)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "timeLog\0".as_ptr() as *const i8,
		call: Some(time_log),
		nargs: 1,
		flags: JSPROP_ENUMERATE as u16,
		jitInfo: ptr::null_mut(),
		usage: "timeLog(label[, exp])\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "timeEnd\0".as_ptr() as *const i8,
		call: Some(time_end),
		nargs: 1,
		flags: JSPROP_ENUMERATE as u16,
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

pub(crate) fn define(cx: *mut JSContext, global: *mut JSObject) -> bool {
	unsafe {
		rooted!(in(cx) let obj = JS_NewPlainObject(cx));
		rooted!(in(cx) let obj_val = ObjectValue(obj.get()));
		rooted!(in(cx) let rooted_global = global);
		return JS_DefineFunctionsWithHelp(cx, obj.handle().into(), METHODS.as_ptr())
			&& JS_DefineProperty(
			cx,
			rooted_global.handle().into(),
			"console\0".as_ptr() as *const i8,
			obj_val.handle().into(),
			0,
		);
	}
}
