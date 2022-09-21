/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Color;

use crate::flags::IteratorFlags;

/// Configuration Object for formatting values
#[derive(Clone, Copy, Debug, Derivative)]
#[derivative(Default)]
pub struct Config {
	pub colors: ColorConfig,
	#[derivative(Default(value = "IteratorFlags::empty()"))]
	pub iteration: IteratorFlags,
	pub depth: u16,
	pub indentation: u16,
	#[derivative(Default(value = "true"))]
	pub multiline: bool,
	pub quoted: bool,
}

impl Config {
	pub fn colors(self, colors: ColorConfig) -> Config {
		Config { colors, ..self }
	}

	pub fn iteration(self, iteration: IteratorFlags) -> Config {
		Config { iteration, ..self }
	}

	pub fn depth(self, depth: u16) -> Config {
		Config { depth, ..self }
	}

	pub fn indentation(self, indentation: u16) -> Config {
		Config { indentation, ..self }
	}

	pub fn multiline(self, multiline: bool) -> Config {
		Config { multiline, ..self }
	}

	pub fn quoted(self, quoted: bool) -> Config {
		Config { quoted, ..self }
	}
}

/// Configuration Object for the colours used when formatting values as specific types.
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
	/// Returns [ColorConfig] where all formatted strings are white.
	pub fn white() -> ColorConfig {
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
