/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;

use crate::{Context, OwnedKey};
use crate::format::Config;
use crate::format::symbol::format_symbol;

/// Formats the [key of an object](OwnedKey) as a string with the given [configuration](Config),
pub fn format_key(cx: &Context, cfg: Config, key: &OwnedKey) -> String {
	match key {
		OwnedKey::Int(i) => i.to_string().color(cfg.colours.object).to_string(),
		OwnedKey::String(str) => format!("\"{}\"", str).color(cfg.colours.string).to_string(),
		OwnedKey::Symbol(symbol) => format!("[{}]", format_symbol(cx, cfg, symbol)).color(cfg.colours.symbol).to_string(),
		OwnedKey::Void => unreachable!("Property key <void> cannot be formatted."),
	}
}
