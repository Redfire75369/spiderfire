/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use indent::indent_by;

use crate::{Context, Function};
use crate::format::Config;

/// Formats a [Function] as a [String], using the given [configuration](Config).
///
/// ### Format
/// ```js
/// function <#name>(<#arguments, ...>) {
///   <#body>
/// }
/// ```
pub fn format_function(cx: &Context, cfg: Config, function: &Function) -> String {
	indent_by((2 * (cfg.indentation + cfg.depth)) as usize, function.to_string(cx))
}
