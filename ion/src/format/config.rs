/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Color;

use crate::flags::IteratorFlags;

/// Configuration for the colours used when formatting values as specific types.
#[derive(Clone, Copy, Debug)]
pub struct ColourConfig {
	pub boolean: Color,
	pub number: Color,
	pub string: Color,
	pub bigint: Color,
	pub symbol: Color,
	pub null: Color,
	pub undefined: Color,
	pub array: Color,
	pub object: Color,
	pub function: Color,
	pub date: Color,
	pub promise: Color,
	pub regexp: Color,
}

impl Default for ColourConfig {
	fn default() -> Self {
		ColourConfig {
			boolean: Color::Cyan,
			number: Color::Blue,
			string: Color::Green,
			bigint: Color::Blue,
			symbol: Color::Magenta,
			null: Color::TrueColor { r: 118, g: 118, b: 118 },
			undefined: Color::TrueColor { r: 118, g: 118, b: 118 },
			array: Color::White,
			object: Color::White,
			function: Color::White,
			date: Color::White,
			promise: Color::Yellow,
			regexp: Color::Green,
		}
	}
}

impl ColourConfig {
	/// Returns [ColourConfig] where all formatted strings are white.
	pub fn white() -> ColourConfig {
		ColourConfig {
			boolean: Color::White,
			number: Color::White,
			string: Color::White,
			bigint: Color::White,
			symbol: Color::White,
			null: Color::White,
			undefined: Color::White,
			array: Color::White,
			object: Color::White,
			function: Color::White,
			date: Color::White,
			promise: Color::White,
			regexp: Color::White,
		}
	}
}

/// Represents configuration for formatting
#[derive(Clone, Copy, Debug)]
pub struct Config {
	pub colours: ColourConfig,
	pub iteration: IteratorFlags,
	pub depth: u16,
	pub indentation: u16,
	pub multiline: bool,
	pub quoted: bool,
}

impl Config {
	/// Replaces the colors in the [configuration](Config).
	pub fn colours(self, colours: ColourConfig) -> Config {
		Config { colours, ..self }
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

impl Default for Config {
	fn default() -> Config {
		Config {
			colours: ColourConfig::default(),
			iteration: IteratorFlags::default(),
			depth: 0,
			indentation: 0,
			multiline: true,
			quoted: false,
		}
	}
}
