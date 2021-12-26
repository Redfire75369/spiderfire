/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::collections::hash_map::{Entry, HashMap};

use chrono::{DateTime, offset::Utc};
use indent::indent_all_by;
use indexmap::IndexSet;
use mozjs::jsapi::{JS_DefineFunctions, JS_NewPlainObject, JSFunctionSpec, StackFormat, Value};
use mozjs::jsval::ObjectValue;
use term_table::{Table, TableStyle};
use term_table::row::Row;
use term_table::table_cell::{Alignment, TableCell};

use ion::{IonContext, IonResult};
use ion::flags::PropertyFlags;
use ion::format::{format_value, INDENT};
use ion::format::config::FormatConfig;
use ion::format::primitive::format_primitive;
use ion::objects::object::{IonObject, Key};

use crate::config::{Config, LogLevel};

const ANSI_CLEAR: &str = "\x1b[1;1H";
const ANSI_CLEAR_SCREEN_DOWN: &str = "\x1b[0J";

const DEFAULT_LABEL: &str = "default";

thread_local! {
	static COUNT_MAP: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
	static TIMER_MAP: RefCell<HashMap<String, DateTime<Utc>>> = RefCell::new(HashMap::new());

	static INDENTS: RefCell<u16> = RefCell::new(0);
}

fn get_indents() -> u16 {
	INDENTS.with(|indents| *indents.borrow())
}

fn print_indent(is_stderr: bool) {
	INDENTS.with(|indents| {
		if !is_stderr {
			print!("{}", INDENT.repeat(*indents.borrow() as usize));
		} else {
			eprint!("{}", INDENT.repeat(*indents.borrow() as usize));
		}
	});
}

fn print_args(cx: IonContext, args: Vec<Value>, stderr: bool) {
	let indents = get_indents();
	for i in 0..args.len() {
		let value = args[i];
		let string = format_value(cx, FormatConfig::default().indentation(indents), value);
		if !stderr {
			print!("{} ", string);
		} else {
			eprint!("{} ", string);
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
fn log(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Info {
		print_indent(false);
		print_args(cx, values, false);
		println!();
	}

	Ok(())
}

#[js_fn]
fn warn(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Warn {
		print_indent(true);
		print_args(cx, values, true);
		println!();
	}

	Ok(())
}

#[js_fn]
fn error(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level >= LogLevel::Error {
		print_indent(true);
		print_args(cx, values, true);
		println!();
	}

	Ok(())
}

#[js_fn]
fn debug(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	if Config::global().log_level == LogLevel::Debug {
		print_indent(false);
		print_args(cx, values, false);
		println!();
	}

	Ok(())
}

#[js_fn]
fn assert(cx: IonContext, assertion: Option<bool>, #[varargs] values: Vec<Value>) -> IonResult<()> {
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
				eprint!("Assertion Failed: {} ", format_primitive(cx, FormatConfig::default(), values[0]));
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
		}
	}

	Ok(())
}

#[js_fn]
fn clear() -> IonResult<()> {
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
		let stack = stack.unwrap();
		println!(
			"{}",
			&stack
				.as_string(Some(((get_indents() + 1) * 2) as usize), StackFormat::SpiderMonkey)
				.unwrap()
		);
	}

	Ok(())
}

#[js_fn]
fn group(cx: IonContext, #[varargs] values: Vec<Value>) -> IonResult<()> {
	INDENTS.with(|indents| {
		let mut indents = indents.borrow_mut();
		*indents = (*indents).min(u16::MAX - 1) + 1;
	});

	if Config::global().log_level >= LogLevel::Info {
		print_args(cx, values, false);
		println!();
	}

	Ok(())
}

#[js_fn]
fn groupEnd() -> IonResult<()> {
	INDENTS.with(|indents| {
		let mut indents = indents.borrow_mut();
		*indents = (*indents).max(1) - 1;
	});

	Ok(())
}

#[js_fn]
fn count(label: Option<String>) -> IonResult<()> {
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
fn countReset(label: Option<String>) -> IonResult<()> {
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
fn time(label: Option<String>) -> IonResult<()> {
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
fn timeLog(cx: IonContext, label: Option<String>, #[varargs] values: Vec<Value>) -> IonResult<()> {
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
fn timeEnd(label: Option<String>) -> IonResult<()> {
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

#[js_fn]
unsafe fn table(cx: IonContext, data: Value, columns: Option<Vec<String>>) -> IonResult<()> {
	fn sort_keys(unsorted: Vec<Key>) -> IndexSet<Key> {
		let mut indexes = IndexSet::<i32>::new();
		let mut headers = IndexSet::<String>::new();

		for key in unsorted.into_iter() {
			match key {
				Key::Int(index) => indexes.insert(index),
				Key::String(header) => headers.insert(header),
				_ => false,
			};
		}

		combine_keys(indexes, headers)
	}

	fn combine_keys(indexes: IndexSet<i32>, headers: IndexSet<String>) -> IndexSet<Key> {
		let mut indexes: Vec<i32> = indexes.into_iter().collect();
		indexes.sort();

		let mut keys: IndexSet<Key> = indexes.into_iter().map(|index| Key::Int(index)).collect();
		keys.extend(headers.into_iter().map(|header| Key::String(header)));
		keys
	}

	let indents = get_indents();
	if let Some(object) = IonObject::from_value(data) {
		let (rows, columns, has_values) = if let Some(columns) = columns {
			let rows = object.keys(cx, None);
			let mut keys = IndexSet::<Key>::new();

			for column in columns.into_iter() {
				let key = match column.parse::<i32>() {
					Ok(int) => Key::Int(int),
					Err(_) => Key::String(column),
				};
				keys.insert(key);
			}

			(sort_keys(rows), sort_keys(keys.into_iter().collect()), false)
		} else {
			let rows = object.keys(cx, None);
			let mut keys = IndexSet::<Key>::new();
			let mut has_values = false;

			for row in rows.iter() {
				let value = object.get(cx, &row.to_string()).unwrap();
				if let Some(object) = IonObject::from_value(value) {
					let obj_keys = object.keys(cx, None);
					keys.extend(obj_keys);
				} else {
					has_values = true;
				}
			}

			(sort_keys(rows), sort_keys(keys.into_iter().collect()), has_values)
		};

		let mut table = Table::new();
		table.style = TableStyle::thin();

		let mut header_row = vec![TableCell::new_with_alignment("Indices", 1, Alignment::Center)];
		let mut headers = columns
			.iter()
			.map(|column| TableCell::new_with_alignment(column, 1, Alignment::Center))
			.collect();
		header_row.append(&mut headers);
		if has_values {
			header_row.push(TableCell::new_with_alignment("Values", 1, Alignment::Center));
		}
		table.add_row(Row::new(header_row));

		for row in rows.iter() {
			let value = object.get(cx, &row.to_string()).unwrap();
			let mut table_row = vec![TableCell::new_with_alignment(row.to_string(), 1, Alignment::Center)];

			if let Some(object) = IonObject::from_value(value) {
				for column in columns.iter() {
					if let Some(value) = object.get(cx, &column.to_string()) {
						let string = format_value(cx, FormatConfig::default().multiline(false).quoted(true), value);
						table_row.push(TableCell::new_with_alignment(string, 1, Alignment::Center))
					} else {
						table_row.push(TableCell::new(""))
					}
				}
				if has_values {
					table_row.push(TableCell::new(""));
				}
			} else {
				for _ in columns.iter() {
					table_row.push(TableCell::new(""))
				}
				if has_values {
					let string = format_value(cx, FormatConfig::default().multiline(false).quoted(true), value);
					table_row.push(TableCell::new_with_alignment(string, 1, Alignment::Center));
				}
			}

			table.add_row(Row::new(table_row));
		}

		println!("{}", indent_all_by((indents * 2) as usize, table.render()))
	} else {
		if Config::global().log_level >= LogLevel::Info {
			print_indent(true);
			println!("{}", format_value(cx, FormatConfig::default().indentation(indents), data));
		}
	}

	Ok(())
}

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
	function_spec!(table, 1),
	JSFunctionSpec::ZERO,
];

pub unsafe fn define(cx: IonContext, mut global: IonObject) -> bool {
	rooted!(in(cx) let console = JS_NewPlainObject(cx));
	return JS_DefineFunctions(cx, console.handle().into(), METHODS.as_ptr())
		&& global.define(cx, "console", ObjectValue(console.get()), PropertyFlags::CONSTANT_ENUMERATED);
}
