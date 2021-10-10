/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::Value;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::IonContext;
use crate::types::{array::is_array, string::to_string};

pub const INDENT: &str = "  ";
pub const NEWLINE: &str = "\n";

/// Prints a [Value] with the appropriate colour and given indentation, to stdout or stderr.
///
/// ### Colours
/// - Number: Blue
/// - Boolean: Cyan
/// - String: Green
/// - Array: #FF7F3F
/// - Object: #F0F0F0
/// - Undefined/Null: #77676
pub fn print_value(cx: IonContext, val: Value, indents: usize, stderr: bool) {
	let mut out = if !stderr {
		StandardStream::stdout(ColorChoice::Auto)
	} else {
		StandardStream::stderr(ColorChoice::Auto)
	};

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
	} else {
		out.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(118, 118, 118)))).unwrap();
	}

	write!(out, "{}", indent(&to_string(cx, val), indents, false)).unwrap();
	out.reset().unwrap();
}

/// Indents a string with the given indentation.
///
/// Indents first line only when `initial_indent` is true.
pub fn indent(string: &str, indents: usize, initial_indent: bool) -> String {
	let mut output = String::new();
	let indent = INDENT.repeat(indents);

	for (i, line) in string.lines().enumerate() {
		if i > 0 {
			output.push_str(NEWLINE);

			if !line.is_empty() {
				output.push_str(&indent);
			}
		} else if initial_indent && !line.is_empty() {
			output.push_str(&indent);
		}

		output.push_str(line);
	}
	output
}
