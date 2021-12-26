/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Color;
use derivative::Derivative;

#[derive(Clone, Copy, Debug, Derivative)]
#[derivative(Default)]
pub struct FormatConfig {
	pub colors: ColorConfig,
	pub depth: u16,
	pub indentation: u16,
	#[derivative(Default(value = "true"))]
	pub multiline: bool,
	pub quoted: bool,
}

impl FormatConfig {
	pub fn depth(self, depth: u16) -> FormatConfig {
		FormatConfig { depth, ..self }
	}

	pub fn indentation(self, indentation: u16) -> FormatConfig {
		FormatConfig { indentation, ..self }
	}

	pub fn multiline(self, multiline: bool) -> FormatConfig {
		FormatConfig { multiline, ..self }
	}

	pub fn quoted(self, quoted: bool) -> FormatConfig {
		FormatConfig { quoted, ..self }
	}
}

#[derive(Clone, Copy, Debug)]
pub struct ColorConfig {
	pub boolean: Color,
	pub number: Color,
	pub string: Color,
	pub null: Color,
	pub undefined: Color,
	pub array: Color,
	pub object: Color,
	pub date: Color,
}

impl Default for ColorConfig {
	fn default() -> Self {
		ColorConfig {
			boolean: Color::Cyan,
			number: Color::Blue,
			string: Color::Green,
			null: Color::TrueColor { r: 118, g: 118, b: 118 },
			undefined: Color::TrueColor { r: 118, g: 118, b: 118 },
			array: Color::White,
			object: Color::White,
			date: Color::White,
		}
	}
}

impl ColorConfig {
	pub fn no_color() -> ColorConfig {
		ColorConfig {
			boolean: Color::White,
			number: Color::White,
			string: Color::White,
			null: Color::White,
			undefined: Color::White,
			array: Color::White,
			object: Color::White,
			date: Color::White,
		}
	}
}
