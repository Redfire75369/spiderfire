/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};

use colored::Colorize;
use indent::indent_by;

use crate::{Context, Function};
use crate::format::Config;

/// Formats a [function](Function), using the given [configuration](Config).
///
/// ### Format
/// ```js
/// function <#name>(<#arguments, ...>) {
///   <#body>
/// }
/// ```
pub fn format_function<'cx>(cx: &'cx Context, cfg: Config, function: &'cx Function<'cx>) -> FunctionDisplay<'cx> {
	FunctionDisplay { cx, function, cfg }
}

#[must_use]
pub struct FunctionDisplay<'cx> {
	cx: &'cx Context,
	function: &'cx Function<'cx>,
	cfg: Config,
}

impl Display for FunctionDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let spaces = 2 * (self.cfg.indentation + self.cfg.depth);
		write!(
			f,
			"{}",
			indent_by(spaces as usize, self.function.to_string(self.cx)).color(self.cfg.colours.function)
		)
	}
}
