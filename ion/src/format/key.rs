/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};

use colored::Colorize;
use itoa::Buffer;

use crate::{Context, OwnedKey};
use crate::format::Config;
use crate::format::symbol::format_symbol;

/// Formats the [key of an object](OwnedKey) as a string with the given [configuration](Config),
pub fn format_key<'cx>(cx: &'cx Context, cfg: Config, key: &'cx OwnedKey<'cx>) -> KeyDisplay<'cx> {
	KeyDisplay { cx, cfg, key }
}

pub struct KeyDisplay<'cx> {
	cx: &'cx Context,
	cfg: Config,
	key: &'cx OwnedKey<'cx>,
}

impl Display for KeyDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let colours = self.cfg.colours;
		match self.key {
			OwnedKey::Int(i) => {
				let mut buffer = Buffer::new();
				write!(f, "{}", buffer.format(*i).color(colours.number))
			}
			OwnedKey::String(str) => write!(f, "{1}{}{1}", r#"""#.color(colours.string), str.color(colours.string)),
			OwnedKey::Symbol(sym) => {
				write!(f, "{}", "[".color(colours.symbol))?;
				write!(f, "{}", format_symbol(self.cx, self.cfg, sym))?;
				write!(f, "{}", "]".color(colours.symbol))
			}
			OwnedKey::Void => unreachable!("Property key <void> cannot be formatted."),
		}
	}
}
