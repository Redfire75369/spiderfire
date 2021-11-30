/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::Value;

use crate::format::config::Config;
use crate::format::object::format_object;
use crate::format::primitive::format_primitive;
use crate::IonContext;

pub mod array;
pub mod boxed;
pub mod config;
pub mod date;
pub mod object;
pub mod primitive;

pub const INDENT: &str = "  ";
pub const NEWLINE: &str = "\n";

/// Formats a [Value] to a [String] using the given configuration options
pub fn format_value(cx: IonContext, cfg: Config, value: Value) -> String {
	if !value.is_object() {
		format_primitive(cx, cfg, value)
	} else {
		format_object(cx, cfg, value)
	}
}
