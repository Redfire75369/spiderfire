/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::io::Write;

use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::*;
use mozjs::rust::RootedGuard;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub(crate) fn print_value(cx: *mut JSContext, rval: RootedGuard<'_, Value>, is_error: bool) {
	let val = rval.get();
	let mut out;
	if !is_error {
		out = StandardStream::stdout(ColorChoice::Auto);
	} else {
		out = StandardStream::stderr(ColorChoice::Auto);
	}

	if val.is_number() {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Blue))).unwrap();
		write!(out, "{}", val.to_number()).unwrap();
	} else if val.is_boolean() {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Cyan))).unwrap();
		write!(out, "{}", val.to_boolean()).unwrap();
	} else if val.is_string() {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
		unsafe {
			write!(out, "\"{}\"", jsstr_to_string(cx, val.to_string())).unwrap();
		}
	} else if val.is_null() {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(118, 118, 118)))).unwrap();
		write!(out, "null").unwrap();
	} else if val.is_object() {
		rooted!(in(cx) let rval = val);

		out.set_color(ColorSpec::new().set_fg(Some(Color::White))).unwrap();
		unsafe {
			write!(out, "{}", jsstr_to_string(cx, JS_ValueToSource(cx, rval.handle().into()))).unwrap();
		}
	} else {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(118, 118, 118)))).unwrap();
		write!(out, "undefined").unwrap();
	}

	out.reset().unwrap();
}

pub(crate) fn println_value(cx: *mut JSContext, rval: RootedGuard<'_, Value>, is_error: bool) {
	print_value(cx, rval, is_error);
	println!();
}
