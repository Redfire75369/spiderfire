/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsval::JSVal;

pub use config::Config;

use crate::Context;
use crate::format::object::format_object;
use crate::format::primitive::format_primitive;

pub mod array;
pub mod boxed;
pub mod class;
mod config;
pub mod date;
pub mod function;
pub mod object;
pub mod primitive;

pub const INDENT: &str = "  ";
pub const NEWLINE: &str = "\n";

/// Formats a [JSVal] as a [String] with the given [Config].
pub fn format_value(cx: Context, cfg: Config, value: JSVal) -> String {
	if !value.is_object() {
		format_primitive(cx, cfg, value)
	} else {
		format_object(cx, cfg, value.to_object())
	}
}
