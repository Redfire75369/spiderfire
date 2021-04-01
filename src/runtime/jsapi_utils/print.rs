/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::io::Write;

use mozjs::jsapi::*;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::runtime::jsapi_utils::{array::is_array, string::to_string};
use crate::utils::indents::indent;

pub(crate) fn print_value(cx: *mut JSContext, val: Value, indents: usize, is_error: bool) {
	let mut out;
	if !is_error {
		out = StandardStream::stdout(ColorChoice::Auto);
	} else {
		out = StandardStream::stderr(ColorChoice::Auto);
	}

	if val.is_number() {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Blue))).unwrap();
	} else if val.is_boolean() {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Cyan))).unwrap();
	} else if val.is_string() {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
	} else if is_array(cx, val) {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(255, 127, 63)))).unwrap();
	} else if val.is_object() {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(240, 240, 240)))).unwrap();
	} else if val.is_null() {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(118, 118, 118)))).unwrap();
	} else {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(118, 118, 118)))).unwrap();
	}

	write!(out, "{}", indent(&to_string(cx, val), indents, false)).unwrap();
	out.reset().unwrap();
}

pub(crate) fn println_value(cx: *mut JSContext, val: Value, indents: usize, is_error: bool) {
	print_value(cx, val, indents, is_error);
	println!();
}
