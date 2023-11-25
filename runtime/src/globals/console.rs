/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::collections::hash_map::{Entry, HashMap};

use chrono::{DateTime, offset::Utc};
use indent::indent_all_by;
use indexmap::IndexSet;
use mozjs::jsapi::JSFunctionSpec;
use term_table::{Table, TableStyle};
use term_table::row::Row;
use term_table::table_cell::{Alignment, TableCell};

use ion::{Context, Object, OwnedKey, Stack, Value};
use ion::conversions::FromValue;
use ion::flags::PropertyFlags;
use ion::format::{format_value, INDENT};
use ion::format::Config as FormatConfig;
use ion::format::key::format_key;
use ion::format::primitive::format_primitive;

use crate::cache::map::find_sourcemap;
use crate::config::{Config, LogLevel};

const ANSI_CLEAR: &str = "\x1b[1;1H";
const ANSI_CLEAR_SCREEN_DOWN: &str = "\x1b[0J";

const DEFAULT_LABEL: &str = "default";

thread_local! {
	static COUNT_MAP: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
	static TIMER_MAP: RefCell<HashMap<String, DateTime<Utc>>> = RefCell::new(HashMap::new());

	static INDENTS: Cell<u16> = Cell::new(0);
}

fn print_indent(is_stderr: bool) {
	let indents = INDENTS.get();
	if !is_stderr {
		print!("{}", INDENT.repeat(indents as usize));
	} else {
		eprint!("{}", INDENT.repeat(indents as usize));
	}
}

fn print_args(cx: &Context, args: &[Value], stderr: bool) {
	for value in args.iter() {
		let string = format_value(cx, FormatConfig::default().indentation(INDENTS.get()), value);
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
fn log(cx: &Context, #[ion(varargs)] values: Vec<Value>) {
	if Config::global().log_level >= LogLevel::Info {
		print_indent(false);
		print_args(cx, values.as_slice(), false);
		println!();
	}
}

#[js_fn]
fn warn(cx: &Context, #[ion(varargs)] values: Vec<Value>) {
	if Config::global().log_level >= LogLevel::Warn {
		print_indent(true);
		print_args(cx, values.as_slice(), true);
		println!();
	}
}

#[js_fn]
fn error(cx: &Context, #[ion(varargs)] values: Vec<Value>) {
	if Config::global().log_level >= LogLevel::Error {
		print_indent(true);
		print_args(cx, values.as_slice(), true);
		println!();
	}
}

#[js_fn]
fn debug(cx: &Context, #[ion(varargs)] values: Vec<Value>) {
	if Config::global().log_level == LogLevel::Debug {
		print_indent(false);
		print_args(cx, values.as_slice(), false);
		println!();
	}
}

#[js_fn]
fn assert(cx: &Context, assertion: Option<bool>, #[ion(varargs)] values: Vec<Value>) {
	if Config::global().log_level >= LogLevel::Error {
		if let Some(assertion) = assertion {
			if assertion {
				return;
			}

			if values.is_empty() {
				print_indent(true);
				eprintln!("Assertion Failed");
				return;
			}

			if values[0].handle().is_string() {
				print_indent(true);
				eprint!(
					"Assertion Failed: {} ",
					format_primitive(cx, FormatConfig::default(), &values[0])
				);
				print_args(cx, &values[2..], true);
				eprintln!();
				return;
			}

			print_indent(true);
			eprint!("Assertion Failed: ");
			print_args(cx, values.as_slice(), true);
			println!();
		} else {
			eprintln!("Assertion Failed:");
		}
	}
}

#[js_fn]
fn clear() {
	INDENTS.set(0);

	println!("{}", ANSI_CLEAR);
	println!("{}", ANSI_CLEAR_SCREEN_DOWN);
}

#[js_fn]
fn trace(cx: &Context, #[ion(varargs)] values: Vec<Value>) {
	if Config::global().log_level == LogLevel::Debug {
		print_indent(false);
		print!("Trace: ");
		print_args(cx, values.as_slice(), false);
		println!();

		let mut stack = Stack::from_capture(cx);
		let indents = ((INDENTS.get() + 1) * 2) as usize;

		if let Some(stack) = &mut stack {
			for record in &mut stack.records {
				if let Some(sourcemap) = find_sourcemap(&record.location.file) {
					record.transform_with_sourcemap(&sourcemap);
				}
			}

			println!("{}", &indent_all_by(indents, stack.format()));
		} else {
			eprintln!("Current Stack could not be captured.");
		}
	}
}

#[js_fn]
fn group(cx: &Context, #[ion(varargs)] values: Vec<Value>) {
	INDENTS.set(INDENTS.get().min(u16::MAX - 1) + 1);

	if Config::global().log_level >= LogLevel::Info {
		print_args(cx, values.as_slice(), false);
		println!();
	}
}

#[js_fn]
fn groupEnd() {
	INDENTS.set(INDENTS.get().max(1) - 1);
}

#[js_fn]
fn count(label: Option<String>) {
	let label = get_label(label);
	COUNT_MAP.with_borrow_mut(|counts| match counts.entry(label.clone()) {
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
	});
}

#[js_fn]
fn countReset(label: Option<String>) {
	let label = get_label(label);
	COUNT_MAP.with_borrow_mut(|counts| match counts.entry(label.clone()) {
		Entry::Vacant(_) => {
			if Config::global().log_level >= LogLevel::Error {
				print_indent(true);
				eprintln!("Count for {} does not exist", label);
			}
		}
		Entry::Occupied(mut o) => {
			o.insert(0);
		}
	});
}

#[js_fn]
fn time(label: Option<String>) {
	let label = get_label(label);
	TIMER_MAP.with_borrow_mut(|timers| match timers.entry(label.clone()) {
		Entry::Vacant(v) => {
			v.insert(Utc::now());
		}
		Entry::Occupied(_) => {
			if Config::global().log_level >= LogLevel::Error {
				print_indent(true);
				eprintln!("Timer {} already exists", label);
			}
		}
	});
}

#[js_fn]
fn timeLog(cx: &Context, label: Option<String>, #[ion(varargs)] values: Vec<Value>) {
	let label = get_label(label);
	TIMER_MAP.with_borrow(|timers| match timers.get(&label) {
		Some(start) => {
			if Config::global().log_level >= LogLevel::Info {
				let duration = Utc::now().timestamp_millis() - start.timestamp_millis();
				print_indent(false);
				print!("{}: {}ms ", label, duration);
				print_args(cx, values.as_slice(), false);
				println!();
			}
		}
		None => {
			if Config::global().log_level >= LogLevel::Error {
				print_indent(true);
				eprintln!("Timer {} does not exist", label);
			}
		}
	});
}

#[js_fn]
fn timeEnd(label: Option<String>) {
	let label = get_label(label);
	TIMER_MAP.with_borrow_mut(|timers| match timers.entry(label.clone()) {
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
	});
}

#[js_fn]
fn table(cx: &Context, data: Value, columns: Option<Vec<String>>) {
	fn sort_keys<'cx, I: IntoIterator<Item = OwnedKey<'cx>>>(cx: &'cx Context, unsorted: I) -> IndexSet<OwnedKey<'cx>> {
		let mut indexes = IndexSet::<i32>::new();
		let mut headers = IndexSet::<String>::new();

		for key in unsorted {
			match key {
				OwnedKey::Int(index) => indexes.insert(index),
				OwnedKey::String(header) => headers.insert(header),
				_ => false,
			};
		}

		combine_keys(cx, indexes, headers)
	}

	fn combine_keys(_: &Context, indexes: IndexSet<i32>, headers: IndexSet<String>) -> IndexSet<OwnedKey> {
		let mut indexes: Vec<i32> = indexes.into_iter().collect();
		indexes.sort_unstable();

		let mut keys: IndexSet<OwnedKey> = indexes.into_iter().map(OwnedKey::Int).collect();
		keys.extend(headers.into_iter().map(OwnedKey::String));
		keys
	}

	let indents = INDENTS.get();
	if let Ok(object) = Object::from_value(cx, &data, true, ()) {
		let (rows, columns, has_values) = if let Some(columns) = columns {
			let rows = object.keys(cx, None).map(|key| key.to_owned_key(cx));
			let mut keys = IndexSet::<OwnedKey>::new();

			for column in columns.into_iter() {
				let key = match column.parse::<i32>() {
					Ok(int) => OwnedKey::Int(int),
					Err(_) => OwnedKey::String(column),
				};
				keys.insert(key);
			}

			(sort_keys(cx, rows), sort_keys(cx, keys.into_iter()), false)
		} else {
			let rows: Vec<_> = object.keys(cx, None).map(|key| key.to_owned_key(cx)).collect();
			let mut keys = IndexSet::<OwnedKey>::new();
			let mut has_values = false;

			for row in &rows {
				let value = object.get(cx, row).unwrap();
				if let Ok(object) = Object::from_value(cx, &value, true, ()) {
					let obj_keys = object.keys(cx, None).map(|key| key.to_owned_key(cx));
					keys.extend(obj_keys);
				} else {
					has_values = true;
				}
			}

			(sort_keys(cx, rows), sort_keys(cx, keys.into_iter()), has_values)
		};

		let mut table = Table::new();
		table.style = TableStyle::thin();

		let mut header_row = vec![TableCell::new_with_alignment("Indices", 1, Alignment::Center)];
		let mut headers = columns
			.iter()
			.map(|column| {
				TableCell::new_with_alignment(format_key(cx, FormatConfig::default(), column), 1, Alignment::Center)
			})
			.collect();
		header_row.append(&mut headers);
		if has_values {
			header_row.push(TableCell::new_with_alignment("Values", 1, Alignment::Center));
		}
		table.add_row(Row::new(header_row));

		for row in rows.iter() {
			let value = object.get(cx, row).unwrap();
			let mut table_row = vec![TableCell::new_with_alignment(
				format_key(cx, FormatConfig::default(), row),
				1,
				Alignment::Center,
			)];

			if let Ok(object) = Object::from_value(cx, &value, true, ()) {
				for column in &columns {
					if let Some(value) = object.get(cx, column) {
						let string = format_value(cx, FormatConfig::default().multiline(false).quoted(true), &value);
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
					let string = format_value(cx, FormatConfig::default().multiline(false).quoted(true), &value);
					table_row.push(TableCell::new_with_alignment(string, 1, Alignment::Center));
				}
			}

			table.add_row(Row::new(table_row));
		}

		println!("{}", indent_all_by((indents * 2) as usize, table.render()))
	} else if Config::global().log_level >= LogLevel::Info {
		print_indent(true);
		println!(
			"{}",
			format_value(cx, FormatConfig::default().indentation(indents), &data)
		);
	}
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

pub fn define(cx: &Context, global: &mut Object) -> bool {
	let mut console = Object::new(cx);
	(unsafe { console.define_methods(cx, METHODS) })
		&& global.define_as(cx, "console", &console, PropertyFlags::CONSTANT_ENUMERATED)
}
