/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;
use indent::indent_with;
use mozjs::jsapi::PromiseState;

use crate::{Context, Promise};
use crate::format::{Config, format_value, INDENT};

// TODO: Add Support for Formatting Fulfilled Values and Rejected Reasons
/// Formats a [Promise] as a string with the given [configuration](Config).
/// ### Format
/// ```js
/// Promise { <#state> }
/// ```
pub fn format_promise(cx: &Context, cfg: Config, promise: &Promise) -> String {
	let state = promise.state();
	let state_string = match state {
		PromiseState::Pending => return "Promise { <pending> }".color(cfg.colours.promise).to_string(),
		PromiseState::Fulfilled => "<fulfilled>",
		PromiseState::Rejected => "<rejected>",
	};
	let state_string = state_string.color(cfg.colours.promise);

	let mut base = "Promise {".color(cfg.colours.promise).to_string();
	let result = promise.result(cx);
	let result_string = format_value(cx, cfg, &result);
	let result_string = result_string.trim_start();

	if result_string.contains("\n") {
		base.push('\n');
		base.push_str(&INDENT.repeat((cfg.indentation + cfg.depth + 1) as usize));
		base.push_str(&state_string.to_string());
		base.push(' ');
		base.push_str(&indent_with(INDENT, result_string));
		base.push_str(&"\n}".color(cfg.colours.promise).to_string());
	} else {
		base.push(' ');
		base.push_str(&state_string.to_string());
		base.push(' ');
		base.push_str(result_string);
		base.push_str(&" }".color(cfg.colours.promise).to_string());
	}

	base
}
