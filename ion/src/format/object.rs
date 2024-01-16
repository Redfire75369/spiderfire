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
use mozjs::jsapi::{ESClass, Type};

use crate::{Array, Context, Date, Exception, Function, Object, Promise, PropertyDescriptor, PropertyKey, RegExp};
use crate::conversions::ToValue;
use crate::format::{indent_str, NEWLINE};
use crate::format::array::format_array;
use crate::format::boxed::format_boxed;
use crate::format::class::format_class_object;
use crate::format::Config;
use crate::format::date::format_date;
use crate::format::descriptor::format_descriptor;
use crate::format::function::format_function;
use crate::format::key::format_key;
use crate::format::promise::format_promise;
use crate::format::regexp::format_regexp;
use crate::format::typedarray::{format_array_buffer, format_typed_array};
use crate::typedarray::{
	ArrayBuffer, ArrayBufferView, ClampedUint8Array, Float32Array, Float64Array, Int16Array, Int32Array, Int8Array,
	Uint16Array, Uint32Array, Uint8Array,
};

/// Formats a [JavaScript Object](Object), depending on its class, using the given [configuration](Config).
/// The object is passed to more specific formatting functions, such as [format_array] and [format_date].
pub fn format_object<'cx>(cx: &'cx Context, cfg: Config, object: Object<'cx>) -> ObjectDisplay<'cx> {
	ObjectDisplay { cx, object, cfg }
}

#[must_use]
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
			ESC::Boolean | ESC::Number | ESC::String | ESC::BigInt => format_boxed(cx, cfg, &self.object).fmt(f),
			ESC::Array => format_array(cx, cfg, &Array::from(cx, object.into_local()).unwrap()).fmt(f),
			ESC::Date => format_date(cx, cfg, &Date::from(cx, object.into_local()).unwrap()).fmt(f),
			ESC::Promise => format_promise(cx, cfg, &Promise::from(object.into_local()).unwrap()).fmt(f),
			ESC::RegExp => format_regexp(cx, cfg, &RegExp::from(cx, object.into_local()).unwrap()).fmt(f),
			ESC::Function => format_function(cx, cfg, &Function::from_object(cx, &self.object).unwrap()).fmt(f),
			ESC::ArrayBuffer => format_array_buffer(cfg, &ArrayBuffer::from(object.into_local()).unwrap()).fmt(f),
			ESC::Error => match Exception::from_object(cx, &self.object) {
				Exception::Error(error) => error.format().fmt(f),
				_ => unreachable!("Expected Error"),
			},
			ESC::Object => format_plain_object(cx, cfg, &self.object).fmt(f),
			ESC::Other => {
				if let Some(view) = ArrayBufferView::from(cx.root_object(object.handle().get())) {
					'view: {
						return match view.view_type() {
							Type::Int8 => format_typed_array(cfg, &Int8Array::from(view.into_local()).unwrap()).fmt(f),
							Type::Uint8 => {
								format_typed_array(cfg, &Uint8Array::from(view.into_local()).unwrap()).fmt(f)
							}
							Type::Int16 => {
								format_typed_array(cfg, &Int16Array::from(view.into_local()).unwrap()).fmt(f)
							}
							Type::Uint16 => {
								format_typed_array(cfg, &Uint16Array::from(view.into_local()).unwrap()).fmt(f)
							}
							Type::Int32 => {
								format_typed_array(cfg, &Int32Array::from(view.into_local()).unwrap()).fmt(f)
							}
							Type::Uint32 => {
								format_typed_array(cfg, &Uint32Array::from(view.into_local()).unwrap()).fmt(f)
							}
							Type::Float32 => {
								format_typed_array(cfg, &Float32Array::from(view.into_local()).unwrap()).fmt(f)
							}
							Type::Float64 => {
								format_typed_array(cfg, &Float64Array::from(view.into_local()).unwrap()).fmt(f)
							}
							Type::Uint8Clamped => {
								format_typed_array(cfg, &ClampedUint8Array::from(view.into_local()).unwrap()).fmt(f)
							}
							_ => break 'view,
						};
					}
				}

				format_class_object(cx, cfg, &self.object).fmt(f)
			}
			_ => self.object.as_value(cx).to_source(cx).to_owned(cx).fmt(f),
		}
	}
}

/// Formats a [JavaScript Object](Object) using the given [configuration](Config).
/// Disregards the class of the object.
pub fn format_plain_object<'cx>(cx: &'cx Context, cfg: Config, object: &'cx Object<'cx>) -> PlainObjectDisplay<'cx> {
	PlainObjectDisplay { cx, object, cfg }
}

#[must_use]
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
				"{}".color(colour).fmt(f)
			} else {
				"{".color(colour).fmt(f)?;

				if self.cfg.multiline {
					f.write_str(NEWLINE)?;
					let inner = indent_str((self.cfg.indentation + self.cfg.depth + 1) as usize);

					for key in keys {
						inner.fmt(f)?;
						let desc = self.object.get_descriptor(self.cx, &key).unwrap();
						write_key_descriptor(f, self.cx, self.cfg, &key, &desc, self.object)?;
						",".color(colour).fmt(f)?;
						f.write_str(NEWLINE)?;
					}

					indent_str((self.cfg.indentation + self.cfg.depth) as usize).fmt(f)?;
				} else {
					f.write_char(' ')?;
					let len = length.clamp(0, 3);

					for (i, key) in keys.enumerate() {
						let desc = self.object.get_descriptor(self.cx, &key).unwrap();
						write_key_descriptor(f, self.cx, self.cfg, &key, &desc, self.object)?;

						if i != len - 1 {
							",".color(colour).fmt(f)?;
							f.write_char(' ')?;
						}
					}

					let remaining = length - len;
					write_remaining(f, remaining, None, colour)?;
				}

				"}".color(colour).fmt(f)
			}
		} else {
			"[Object]".color(colour).fmt(f)
		}
	}
}

fn write_key_descriptor(
	f: &mut Formatter, cx: &Context, cfg: Config, key: &PropertyKey, desc: &PropertyDescriptor, object: &Object,
) -> fmt::Result {
	format_key(cx, cfg, &key.to_owned_key(cx)).fmt(f)?;
	": ".color(cfg.colours.object).fmt(f)?;
	format_descriptor(cx, cfg, desc, Some(object)).fmt(f)
}

pub(crate) fn write_remaining(f: &mut Formatter, remaining: usize, inner: Option<&str>, colour: Color) -> fmt::Result {
	if remaining > 0 {
		if let Some(inner) = inner {
			f.write_str(inner)?;
		}

		match remaining.cmp(&1) {
			Ordering::Equal => "... 1 more item".color(colour).fmt(f)?,
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
			",".color(colour).fmt(f)?;
		} else {
			f.write_char(' ')?;
		}
	}
	Ok(())
}
