/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};

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
pub fn format_symbol<'cx>(cx: &'cx Context, cfg: Config, symbol: &'cx Symbol<'cx>) -> SymbolDisplay<'cx> {
	SymbolDisplay { cx, symbol, cfg }
}

pub struct SymbolDisplay<'cx> {
	cx: &'cx Context,
	symbol: &'cx Symbol<'cx>,
	cfg: Config,
}

impl Display for SymbolDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let colour = self.cfg.colours.symbol;
		let code = self.symbol.code();

		match code {
			SymbolCode::WellKnown(code) => write!(f, "{}{}", "Symbol.".color(colour), code.identifier().color(colour)),
			code => {
				let description = self
					.symbol
					.description(self.cx)
					.expect("Expected Description on Non-Well-Known Symbol")
					.color(colour);

				match code {
					SymbolCode::PrivateNameSymbol => return write!(f, "{}", description),
					SymbolCode::InSymbolRegistry => write!(f, "{}", "Symbol.for(".color(colour),)?,
					SymbolCode::UniqueSymbol => write!(f, "{}", "Symbol(".color(colour),)?,
					_ => unreachable!(),
				}

				write!(f, "{}", description)?;
				write!(f, "{}", ")".color(colour))
			}
		}
	}
}
