/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;

use crate::{Context, Symbol};
use crate::format::Config;
use crate::symbol::SymbolCode;

/// Formats a [Symbol] as a [String] with the given [configuration](Config).
///
/// ### Format
/// Well-Known Symbols such as `@@iterator` are formatted as `Symbol.iterator`.
/// Unique Symbols are formatted as `Symbol(<#symbol>)`.
/// Registry Symbols are formatted as `Symbol.for(<#symbol>)`.
/// Private Name Symbols are formatted as `#private`.
pub fn format_symbol(cx: &Context, cfg: Config, symbol: &Symbol) -> String {
	let code = symbol.code();
	match code {
		SymbolCode::WellKnown(code) => format!("Symbol.{}", code.identifier()).color(cfg.colours.symbol).to_string(),
		SymbolCode::PrivateNameSymbol => symbol.description(cx).expect("Expected Description on Private Name Symbol"),
		code => {
			let description = symbol.description(cx).expect("Expected Description on Non-Well-Known Symbol");
			let description = format!("{}{}", description.color(cfg.colours.string), ")".color(cfg.colours.symbol));

			match code {
				SymbolCode::InSymbolRegistry => format!("{}{}", "Symbol.for(".color(cfg.colours.symbol), description),
				SymbolCode::UniqueSymbol => format!("{}{}", "Symbol(".color(cfg.colours.symbol), description),
				_ => unreachable!(),
			}
		}
	}
}
