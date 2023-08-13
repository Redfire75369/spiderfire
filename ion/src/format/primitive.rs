/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;
use mozjs::conversions::jsstr_to_string;

use crate::{Context, Symbol, Value};
use crate::format::Config;
use crate::format::symbol::format_symbol;

/// Formats a primitive value as a string using the given [configuration](Config).
/// The supported types are `boolean`, `number`, `string`, `symbol`, `null` and `undefined`.
pub fn format_primitive(cx: &Context, cfg: Config, value: &Value) -> String {
	let colors = cfg.colours;

	if value.is_boolean() {
		value.to_boolean().to_string().color(colors.boolean).to_string()
	} else if value.is_number() {
		let number = value.to_number();

		if number == f64::INFINITY {
			"Infinity".color(colors.number).to_string()
		} else if number == f64::NEG_INFINITY {
			"-Infinity".color(colors.number).to_string()
		} else {
			number.to_string().color(colors.number).to_string()
		}
	} else if value.is_string() {
		let str = unsafe { jsstr_to_string(**cx, value.to_string()) };
		if cfg.quoted {
			format!("\"{}\"", str).color(colors.string).to_string()
		} else {
			str
		}
	} else if value.is_null() {
		"null".color(colors.null).to_string()
	} else if value.is_undefined() {
		"undefined".color(colors.undefined).to_string()
	} else if value.is_symbol() {
		let symbol = Symbol::from(cx.root_symbol(value.to_symbol()));
		format_symbol(cx, cfg, &symbol)
	} else if value.is_magic() {
		"<magic>".color(colors.boolean).to_string()
	} else {
		unreachable!("Internal Error: Expected Primitive")
	}
}
