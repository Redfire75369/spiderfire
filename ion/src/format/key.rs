/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;

use crate::{Context, Key};
use crate::format::Config;
use crate::format::symbol::format_symbol;

pub fn format_key(cx: &Context, cfg: Config, key: &Key) -> String {
	match key {
		Key::Int(i) => i.to_string().color(cfg.colors.object).to_string(),
		Key::String(str) => format!("\"{}\"", str).color(cfg.colors.string).to_string(),
		Key::Symbol(symbol) => format!("[{}]", format_symbol(cx, cfg, symbol)).color(cfg.colors.symbol).to_string(),
		Key::Void => "<void>".color(cfg.colors.object).to_string(),
	}
}
