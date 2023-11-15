/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter, Write};

use colored::Colorize;
use mozjs::jsapi::ESClass;

use crate::{Array, Context, Date, Exception, Function, Object, Promise, RegExp};
use crate::conversions::ToValue;
use crate::format::{format_value, INDENT, NEWLINE};
use crate::format::array::format_array;
use crate::format::boxed::format_boxed;
use crate::format::class::format_class_object;
use crate::format::Config;
use crate::format::date::format_date;
use crate::format::function::format_function;
use crate::format::key::format_key;
use crate::format::promise::format_promise;
use crate::format::regexp::format_regexp;

/// Formats a [JavaScript Object](Object), depending on its class, as a string using the given [configuration](Config).
/// The object is passed to more specific formatting functions, such as [format_array] and [format_date].
pub fn format_object<'cx>(cx: &'cx Context, cfg: Config, object: Object<'cx>) -> ObjectDisplay<'cx> {
	ObjectDisplay { cx, object, cfg }
}

pub struct ObjectDisplay<'cx> {
	cx: &'cx Context,
	object: Object<'cx>,
	cfg: Config,
}

impl Display for ObjectDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		use ESClass as ESC;

		let cx = self.cx;
		let cfg = self.cfg;
		let object = Object::from(cx.root_object(self.object.handle().get()));

		let class = self.object.get_builtin_class(cx);

		match class {
			ESC::Boolean | ESC::Number | ESC::String | ESC::BigInt => write!(f, "{}", format_boxed(cx, cfg, &self.object)),
			ESC::Array => write!(f, "{}", format_array(cx, cfg, &Array::from(cx, object.into_local()).unwrap())),
			ESC::Object => write!(f, "{}", format_plain_object(cx, cfg, &Object::from(object.into_local()))),
			ESC::Date => write!(f, "{}", format_date(cx, cfg, &Date::from(cx, object.into_local()).unwrap())),
			ESC::Promise => write!(f, "{}", format_promise(cx, cfg, &Promise::from(object.into_local()).unwrap())),
			ESC::RegExp => write!(f, "{}", format_regexp(cx, cfg, &RegExp::from(cx, object.into_local()).unwrap())),
			ESC::Function => write!(f, "{}", format_function(cx, cfg, &Function::from_object(cx, &self.object).unwrap())),
			ESC::Other => write!(f, "{}", format_class_object(cx, cfg, &self.object)),
			ESC::Error => match Exception::from_object(cx, &self.object) {
				Exception::Error(error) => f.write_str(&error.format()),
				_ => unreachable!("Expected Error"),
			},
			_ => {
				let source = self.object.as_value(cx).to_source(cx).to_owned(cx);
				f.write_str(&source)
			}
		}
	}
}

/// Formats a [JavaScript Object](Object) as a string using the given [configuration](Config).
/// Disregards the class of the object.
#[allow(clippy::unnecessary_to_owned)]
pub fn format_plain_object(cx: &Context, cfg: Config, object: &Object) -> String {
	let color = cfg.colours.object;
	if cfg.depth < 4 {
		let keys = object.keys(cx, Some(cfg.iteration));
		let length = keys.len();

		if length == 0 {
			"{}".color(color).to_string()
		} else if cfg.multiline {
			let mut string = format!("{{{}", NEWLINE).color(color).to_string();

			let inner_indent = INDENT.repeat((cfg.indentation + cfg.depth + 1) as usize);
			let outer_indent = INDENT.repeat((cfg.indentation + cfg.depth) as usize);
			for (i, key) in keys.enumerate().take(length) {
				let value = object.get(cx, &key).unwrap();
				let value_string = format_value(cx, cfg.depth(cfg.depth + 1).quoted(true), &value);
				string.push_str(&inner_indent);
				write!(string, "{}: {}", format_key(cx, cfg, &key.to_owned_key(cx)), value_string).unwrap();

				if i != length - 1 {
					string.push_str(&",".color(color).to_string());
				}
				string.push_str(NEWLINE);
			}

			string.push_str(&outer_indent);
			string.push_str(&"}".color(color).to_string());
			string
		} else {
			let mut string = "{ ".color(color).to_string();
			let len = length.clamp(0, 3);
			for (i, key) in keys.enumerate().take(len) {
				let value = object.get(cx, &key).unwrap();
				let value_string = format_value(cx, cfg.depth(cfg.depth + 1).quoted(true), &value);
				write!(string, "{}: {}", format_key(cx, cfg, &key.to_owned_key(cx)), value_string).unwrap();

				if i != len - 1 {
					string.push_str(&", ".color(color).to_string());
				}
			}

			let remaining = length - len;
			match remaining.cmp(&1) {
				Ordering::Equal => string.push_str(&"... 1 more item ".color(color)),
				Ordering::Greater => string.push_str(&format!("... {} more items ", remaining).color(color)),
				_ => (),
			}
			string.push_str(&"}".color(color).to_string());

			string
		}
	} else {
		"[Object]".color(color).to_string()
	}
}
