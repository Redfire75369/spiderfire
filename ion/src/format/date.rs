/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;

use crate::format::config::Config;
use crate::IonContext;
use crate::objects::date::IonDate;

/// Formats an [IonDate] to a [String] using the given configuration options
pub fn format_date(cx: IonContext, cfg: Config, date: IonDate) -> String {
	unsafe {
		if let Some(date) = date.to_date(cx) {
			date.to_string().color(cfg.colors.date).to_string()
		} else {
			panic!("Failed to unbox Date");
		}
	}
}
