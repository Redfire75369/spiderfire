/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;

use crate::{Context, Date};
use crate::format::Config;

/// Formats a [Date] as a [String] using the given [Config]
pub fn format_date(cx: &Context, cfg: Config, date: &Date) -> String {
	if let Some(date) = date.to_date(cx) {
		date.to_string().color(cfg.colors.date).to_string()
	} else {
		panic!("Failed to unbox Date");
	}
}
