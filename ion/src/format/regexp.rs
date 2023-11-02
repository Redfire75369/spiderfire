/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;

use crate::Context;
use crate::format::Config;
use crate::objects::RegExp;

/// Formats a [RegExp object](RegExp) as a string using the given [configuration](Config).
pub fn format_regexp(cx: &Context, cfg: Config, regexp: &RegExp) -> String {
	regexp.to_string(cx).color(cfg.colours.regexp).to_string()
}
