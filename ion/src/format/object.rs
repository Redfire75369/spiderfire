/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::fmt::Write;

use colored::Colorize;
use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{ESClass, GetBuiltinClass, JS_ValueToSource, JSObject};
use mozjs::jsval::ObjectValue;

use crate::{Context, Object};
use crate::flags::IteratorFlags;
use crate::format::{format_value, INDENT, NEWLINE};
use crate::format::array::format_array;
use crate::format::boxed::format_boxed;
use crate::format::class::format_class_object;
use crate::format::Config;
use crate::format::date::format_date;
use crate::format::function::format_function;

/// Formats an [Object], depending on its class, as a [String] using the given [Config].
/// The object is passed to other formatting functions such as [format_array] and [format_date].
pub fn format_object(cx: Context, cfg: Config, object: *mut JSObject) -> String {
	rooted!(in(cx) let robj = object);

	unsafe {
		use ESClass::*;
		let mut class = Other;
		if !GetBuiltinClass(cx, robj.handle().into(), &mut class) {
			return std::string::String::from("");
		}

		match class {
			Boolean | Number | String | BigInt => format_boxed(cx, cfg, object, class),
			Array => format_array(cx, cfg, crate::Array::from(cx, object).unwrap()),
			Object => format_object_raw(cx, cfg, crate::Object::from(object)),
			Date => format_date(cx, cfg, crate::Date::from(cx, object).unwrap()),
			Function => format_function(cx, cfg, crate::Function::from_object(object).unwrap()),
			Other => format_class_object(cx, cfg, crate::Object::from(object)),
			_ => {
				rooted!(in(cx) let rval = ObjectValue(object));
				jsstr_to_string(cx, JS_ValueToSource(cx, rval.handle().into()))
			}
		}
	}
}

/// Formats an [Object] as a [String] using the given [Config].
/// Disregards the class of the object.
pub fn format_object_raw(cx: Context, cfg: Config, object: Object) -> String {
	let color = cfg.colors.object;
	if cfg.depth < 4 {
		let keys = object.keys(cx, Some(IteratorFlags::empty()));
		let length = keys.len();

		if length == 0 {
			"{}".color(color).to_string()
		} else if cfg.multiline {
			let mut string = format!("{{{}", NEWLINE).color(color).to_string();

			let inner_indent = INDENT.repeat((cfg.indentation + cfg.depth + 1) as usize);
			let outer_indent = INDENT.repeat((cfg.indentation + cfg.depth) as usize);
			for (i, key) in keys.into_iter().enumerate().take(length) {
				let value = object.get(cx, &key.to_string()).unwrap();
				let value_string = format_value(cx, cfg.depth(cfg.depth + 1).quoted(true), value);
				string.push_str(&inner_indent);
				write!(string, "{}: {}", key.to_string().color(color), value_string).unwrap();

				if i != length - 1 {
					string.push_str(&",".color(color));
				}
				string.push_str(NEWLINE);
			}

			string.push_str(&outer_indent);
			string.push_str(&"}".color(color));
			string
		} else {
			let mut string = "{ ".color(color).to_string();
			let len = length.clamp(0, 3);
			for (i, key) in keys.into_iter().enumerate().take(len) {
				let value = object.get(cx, &key.to_string()).unwrap();
				let value_string = format_value(cx, cfg.depth(cfg.depth + 1).quoted(true), value);
				write!(string, "{}: {}", key.to_string().color(color), value_string).unwrap();

				if i != len - 1 {
					string.push_str(&", ".color(color));
				}
			}

			let remaining = length - len;
			match remaining.cmp(&1) {
				Ordering::Equal => string.push_str(&"... 1 more item ".color(color)),
				Ordering::Greater => string.push_str(&format!("... {} more items ", remaining).color(color)),
				_ => (),
			}
			string.push_str(&"}".color(color));

			string
		}
	} else {
		"[Object]".color(color).to_string()
	}
}
