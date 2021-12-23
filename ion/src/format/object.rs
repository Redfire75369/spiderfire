/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;
use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{ESClass, GetBuiltinClass, JS_ValueToSource, Value};
use mozjs::jsval::ObjectValue;

use crate::format::{format_value, INDENT, NEWLINE};
use crate::format::array::format_array;
use crate::format::boxed::format_boxed;
use crate::format::class::format_class_object;
use crate::format::config::Config;
use crate::format::date::format_date;
use crate::format::function::format_function;
use crate::functions::function::IonFunction;
use crate::IonContext;
use crate::objects::array::IonArray;
use crate::objects::date::IonDate;
use crate::objects::object::IonObject;

/// Formats an object to a [String] using the given configuration options
pub fn format_object(cx: IonContext, cfg: Config, value: Value) -> String {
	assert!(value.is_object());
	let object = value.to_object();
	rooted!(in(cx) let robj = object);

	unsafe {
		use ESClass::*;
		let mut class = Other;
		if !GetBuiltinClass(cx, robj.handle().into(), &mut class) {
			return std::string::String::from("");
		}

		match class {
			Boolean | Number | String | BigInt => format_boxed(cx, cfg, object, class),
			Array => format_array(cx, cfg, IonArray::from(cx, object).unwrap()),
			Object => format_object_raw(cx, cfg, IonObject::from(object)),
			Date => format_date(cx, cfg, IonDate::from(cx, object).unwrap()),
			Function => format_function(cx, cfg, IonFunction::from_object(cx, object).unwrap()),
			Other => format_class_object(cx, cfg, IonObject::from(object)),
			_ => {
				rooted!(in(cx) let rval = ObjectValue(object));
				jsstr_to_string(cx, JS_ValueToSource(cx, rval.handle().into()))
			}
		}
	}
}

pub fn format_object_raw(cx: IonContext, cfg: Config, object: IonObject) -> String {
	let color = cfg.colors.object;
	if cfg.depth < 4 {
		unsafe {
			let mut keys = object.keys(cx, Some(0));
			let length = keys.len();
			keys.sort();

			if length == 0 {
				"{}".color(color).to_string()
			} else {
				let mut string = format!("{{{}", NEWLINE).color(color).to_string();

				let inner_indent = INDENT.repeat((cfg.indentation + cfg.depth + 1) as usize);
				let outer_indent = INDENT.repeat((cfg.indentation + cfg.depth) as usize);
				for i in 0..length {
					let key = keys[i].clone();
					let value = object.get(cx, &key).unwrap();
					let value_string = format_value(cx, cfg.depth(cfg.depth + 1).quoted(true), value);
					string.push_str(&inner_indent);
					string.push_str(&format!("{}: {}", key.color(color), value_string));

					if i != length - 1 {
						string.push_str(&format!(",{}", NEWLINE).color(color).to_string());
					} else {
						string.push_str(NEWLINE);
					}
				}

				string.push_str(&outer_indent);
				string.push_str(&"}".color(color).to_string());
				string
			}
		}
	} else {
		"[Object]".color(color).to_string()
	}
}
