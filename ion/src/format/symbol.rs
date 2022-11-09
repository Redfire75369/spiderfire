/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;

use crate::{Context, Symbol};
use crate::format::Config;
use crate::symbol::SymbolCode;

pub fn format_symbol(cx: &Context, cfg: Config, symbol: &Symbol) -> String {
	let code = symbol.code();
	match code {
		SymbolCode::WellKnown(code) => format!("Symbol.{}", code.identifier()).color(cfg.colors.symbol).to_string(),
		SymbolCode::PrivateNameSymbol => symbol.description(cx).expect("Expected Description on Private Name Symbol"),
		code => {
			let description = symbol.description(cx).expect("Expected Description on Non-Well-Known Symbol");
			let description = format!("{}{}", description.color(cfg.colors.string), ")".color(cfg.colors.symbol));

			match code {
				SymbolCode::InSymbolRegistry => format!("{}{}", "Symbol.for(".color(cfg.colors.symbol), description),
				SymbolCode::UniqueSymbol => format!("{}{}", "Symbol(".color(cfg.colors.symbol), description),
				_ => unreachable!(),
			}
		}
	}
}
