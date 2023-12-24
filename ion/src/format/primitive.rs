/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};

use colored::Colorize;
use itoa::Buffer;

use crate::{Context, Symbol, Value};
use crate::bigint::BigInt;
use crate::conversions::FromValue;
use crate::format::Config;
use crate::format::symbol::format_symbol;

/// Formats a primitive value as a string using the given [configuration](Config).
/// The supported types are `boolean`, `number`, `string`, `symbol`, `null` and `undefined`.
pub fn format_primitive<'cx>(cx: &'cx Context, cfg: Config, value: &'cx Value<'cx>) -> PrimitiveDisplay<'cx> {
	PrimitiveDisplay { cx, value, cfg }
}

pub struct PrimitiveDisplay<'cx> {
	cx: &'cx Context,
	value: &'cx Value<'cx>,
	cfg: Config,
}

impl Display for PrimitiveDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let colours = self.cfg.colours;

		let value = self.value.handle();
		if value.is_boolean() {
			write!(f, "{}", value.to_boolean().to_string().color(colours.boolean))
		} else if value.is_int32() {
			let int = value.to_int32();
			let mut buffer = Buffer::new();
			write!(f, "{}", buffer.format(int).color(colours.number))
		} else if value.is_double() {
			let number = value.to_double();

			if number == f64::INFINITY {
				write!(f, "{}", "Infinity".color(colours.number))
			} else if number == f64::NEG_INFINITY {
				write!(f, "{}", "-Infinity".color(colours.number))
			} else {
				write!(f, "{}", number.to_string().color(colours.number))
			}
		} else if value.is_string() {
			let str = crate::String::from_value(self.cx, self.value, true, ()).unwrap().to_owned(self.cx);
			if self.cfg.quoted {
				write!(f, "{0}{1}{0}", r#"""#.color(colours.string), str.color(colours.string))
			} else {
				f.write_str(&str)
			}
		} else if value.is_null() {
			write!(f, "{}", "null".color(colours.null))
		} else if value.is_undefined() {
			write!(f, "{}", "undefined".color(colours.undefined))
		} else if value.is_bigint() {
			let bi = BigInt::from(self.cx.root_bigint(value.to_bigint()));
			write!(
				f,
				"{}{}",
				bi.to_string(self.cx, 10).unwrap().to_owned(self.cx).color(colours.bigint),
				"n".color(colours.bigint)
			)
		} else if value.is_symbol() {
			let symbol = Symbol::from(self.cx.root_symbol(value.to_symbol()));
			write!(f, "{}", format_symbol(self.cx, self.cfg, &symbol))
		} else if value.is_magic() {
			write!(f, "{}", "<magic>".color(colours.boolean))
		} else {
			unreachable!("Expected Primitive")
		}
	}
}
