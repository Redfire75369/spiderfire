/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;
use mozjs::jsapi::JS::PromiseState;

use crate::{Context, Promise};
use crate::format::Config;

// TODO: Add Support for Formatting Fulfilled Values and Rejected Reasons
/// Formats a [Promise] as a string with the given [configuration](Config).
/// ### Format
/// ```js
/// Promise { <#state> }
/// ```
pub fn format_promise(_cx: &Context, cfg: Config, promise: &Promise) -> String {
	let state = promise.state();
	let base = match state {
		PromiseState::Pending => "Promise { <pending> }",
		PromiseState::Fulfilled => "Promise { <fulfilled> }",
		PromiseState::Rejected => "Promise { <rejected> }",
	};
	base.color(cfg.colours.promise).to_string()
}
