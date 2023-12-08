/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter, Write};

use colored::{Color, Colorize};
use itoa::Buffer;
use mozjs::jsapi::ESClass;

use crate::{Array, Context, Date, Exception, Function, Object, Promise, PropertyKey, RegExp, Value};
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
use crate::format::typedarray::format_array_buffer;
use crate::typedarray::buffer::ArrayBuffer;

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
			ESC::Boolean | ESC::Number | ESC::String | ESC::BigInt => {
				write!(f, "{}", format_boxed(cx, cfg, &self.object))
			}
			ESC::Array => write!(
				f,
				"{}",
				format_array(cx, cfg, &Array::from(cx, object.into_local()).unwrap())
			),
			ESC::Object => write!(
				f,
				"{}",
				format_plain_object(cx, cfg, &Object::from(object.into_local()))
			),
			ESC::Date => write!(
				f,
				"{}",
				format_date(cx, cfg, &Date::from(cx, object.into_local()).unwrap())
			),
			ESC::Promise => write!(
				f,
				"{}",
				format_promise(cx, cfg, &Promise::from(object.into_local()).unwrap())
			),
			ESC::RegExp => write!(
				f,
				"{}",
				format_regexp(cx, cfg, &RegExp::from(cx, object.into_local()).unwrap())
			),
			ESC::Function => write!(
				f,
				"{}",
				format_function(cx, cfg, &Function::from_object(cx, &self.object).unwrap())
			),
			ESC::ArrayBuffer => write!(
				f,
				"{}",
				format_array_buffer(cfg, &ArrayBuffer::from(object.into_local()).unwrap())
			),
			ESC::Error => match Exception::from_object(cx, &self.object) {
				Exception::Error(error) => f.write_str(&error.format()),
				_ => unreachable!("Expected Error"),
			},
			ESC::Other => write!(f, "{}", format_class_object(cx, cfg, &self.object)),
			_ => {
				let source = self.object.as_value(cx).to_source(cx).to_owned(cx);
				f.write_str(&source)
			}
		}
	}
}

/// Formats a [JavaScript Object](Object) as a string using the given [configuration](Config).
/// Disregards the class of the object.
pub fn format_plain_object<'cx>(cx: &'cx Context, cfg: Config, object: &'cx Object<'cx>) -> PlainObjectDisplay<'cx> {
	PlainObjectDisplay { cx, object, cfg }
}

pub struct PlainObjectDisplay<'cx> {
	cx: &'cx Context,
	object: &'cx Object<'cx>,
	cfg: Config,
}

impl Display for PlainObjectDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let colour = self.cfg.colours.object;

		if self.cfg.depth < 4 {
			let keys = self.object.keys(self.cx, Some(self.cfg.iteration));
			let length = keys.len();

			if length == 0 {
				write!(f, "{}", "{}".color(colour))
			} else {
				write!(f, "{}", "{".color(colour))?;

				if self.cfg.multiline {
					f.write_str(NEWLINE)?;
					let inner = INDENT.repeat((self.cfg.indentation + self.cfg.depth + 1) as usize);

					for key in keys {
						f.write_str(&inner)?;
						let value = self.object.get(self.cx, &key).unwrap();
						write_key_value(f, self.cx, self.cfg, &key, &value)?;
						write!(f, "{}", ",".color(colour))?;
						f.write_str(NEWLINE)?;
					}

					f.write_str(&INDENT.repeat((self.cfg.indentation + self.cfg.depth) as usize))?;
				} else {
					f.write_char(' ')?;
					let len = length.clamp(0, 3);

					for (i, key) in keys.enumerate() {
						let value = self.object.get(self.cx, &key).unwrap();
						write_key_value(f, self.cx, self.cfg, &key, &value)?;

						if i != len - 1 {
							write!(f, "{}", ",".color(colour))?;
							f.write_char(' ')?;
						}
					}

					let remaining = length - len;
					write_remaining(f, remaining, None, colour)?;
				}

				write!(f, "{}", "}".color(colour))
			}
		} else {
			write!(f, "{}", "[Object]".color(colour))
		}
	}
}

fn write_key_value(f: &mut Formatter, cx: &Context, cfg: Config, key: &PropertyKey, value: &Value) -> fmt::Result {
	write!(
		f,
		"{}{} {}",
		format_key(cx, cfg, &key.to_owned_key(cx)),
		":".color(cfg.colours.object),
		format_value(cx, cfg.depth(cfg.depth + 1).quoted(true), value)
	)
}

pub(crate) fn write_remaining(f: &mut Formatter, remaining: usize, inner: Option<&str>, colour: Color) -> fmt::Result {
	if remaining > 0 {
		if let Some(inner) = inner {
			write!(f, "{}", inner)?;
		}

		match remaining.cmp(&1) {
			Ordering::Equal => write!(f, "{}", "... 1 more item".color(colour))?,
			Ordering::Greater => {
				let mut buffer = Buffer::new();
				write!(
					f,
					"{} {} {}",
					"...".color(colour),
					buffer.format(remaining).color(colour),
					"more items".color(colour)
				)?
			}
			_ => (),
		}
		if inner.is_some() {
			write!(f, "{}", ",".color(colour))?;
		} else {
			f.write_char(' ')?;
		}
	}
	Ok(())
}
