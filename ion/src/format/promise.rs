/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter, Write};

use colored::Colorize;
use mozjs::jsapi::PromiseState;

use crate::{Context, Promise};
use crate::format::{Config, format_value, INDENT};

/// Formats a [Promise] as a string with the given [configuration](Config).
/// ### Format
/// ```js
/// Promise { <#state> <#result> }
/// ```
pub fn format_promise<'cx>(cx: &'cx Context, cfg: Config, promise: &'cx Promise) -> PromiseDisplay<'cx> {
	PromiseDisplay { cx, promise, cfg }
}

pub struct PromiseDisplay<'cx> {
	cx: &'cx Context,
	promise: &'cx Promise,
	cfg: Config,
}

impl Display for PromiseDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let colour = self.cfg.colours.promise;
		let state = self.promise.state();

		let state = match state {
			PromiseState::Pending => return write!(f, "{}", "Promise { <pending> }".color(colour)),
			PromiseState::Fulfilled => "<fulfilled>".color(colour),
			PromiseState::Rejected => "<rejected>".color(colour),
		};

		write!(f, "{}", "Promise {".color(colour))?;
		let result = self.promise.result(self.cx);

		if self.cfg.multiline {
			let result = format_value(self.cx, self.cfg.depth(self.cfg.depth + 1), &result);

			f.write_char('\n')?;
			f.write_str(&INDENT.repeat((self.cfg.indentation + self.cfg.depth + 1) as usize))?;
			write!(f, "{}", state)?;
			f.write_char(' ')?;
			write!(f, "{}", result)?;
			write!(f, "{}", "\n}".color(colour))
		} else {
			let result = format_value(self.cx, self.cfg, &result);

			f.write_char(' ')?;
			write!(f, "{}", state)?;
			f.write_char(' ')?;
			write!(f, "{}", result)?;
			write!(f, "{}", " }".color(colour))
		}
	}
}
