/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use colored::Colorize;

use crate::format::{format_value, INDENT, NEWLINE};
use crate::format::config::FormatConfig;
use crate::IonContext;
use crate::objects::array::IonArray;

/// Formats an [IonArray] to a [String] using the given configuration options
pub fn format_array(cx: IonContext, cfg: FormatConfig, array: IonArray) -> String {
	let color = cfg.colors.array;
	if cfg.depth < 5 {
		unsafe {
			let vec = array.to_vec(cx);
			let length = vec.len();

			if length == 0 {
				"[]".color(color).to_string()
			} else if cfg.multiline {
				let mut string = format!("[{}", NEWLINE).color(color).to_string();
				let len = length.clamp(0, 100);
				let remaining = length - len;

				let inner_indent = INDENT.repeat((cfg.indentation + cfg.depth + 1) as usize);
				let outer_indent = INDENT.repeat((cfg.indentation + cfg.depth) as usize);
				for i in 0..len {
					let value = vec[i];
					let value_string = format_value(cx, cfg.depth(cfg.depth + 1).quoted(true), value);
					string.push_str(&inner_indent);
					string.push_str(&value_string);

					if i != length - 1 {
						string.push_str(&format!(",{}", NEWLINE).color(color).to_string());
					} else {
						string.push_str(NEWLINE);
					}
				}

				if remaining > 0 {
					string.push_str(&inner_indent);
					if remaining == 1 {
						string.push_str(&"... 1 more item".color(color).to_string());
					} else if remaining > 1 {
						string.push_str(&format!("... {} more items", remaining).color(color).to_string());
					}
				}

				string.push_str(&outer_indent);
				string.push_str(&"]".color(color).to_string());
				string
			} else {
				let mut string = "[ ".color(color).to_string();
				let len = length.clamp(0, 3);

				for i in 0..len {
					let value = vec[i];
					let value_string = format_value(cx, cfg.depth(cfg.depth + 1).quoted(true), value);
					string.push_str(&value_string);

					if i != len - 1 {
						string.push_str(&", ".color(color).to_string());
					}
				}

				let remaining = length - len;
				if remaining == 1 {
					string.push_str(&", ... 1 more item ".color(color).to_string());
				} else if remaining > 1 {
					string.push_str(&format!(", ... {} more items ", remaining).color(color).to_string());
				}
				string.push_str(&"]".color(color).to_string());

				string
			}
		}
	} else {
		"[Array]".color(color).to_string()
	}
}
