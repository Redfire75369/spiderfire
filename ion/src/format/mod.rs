/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub use config::Config;

use crate::{Context, Value};
use crate::format::object::format_object;
use crate::format::primitive::format_primitive;

pub mod array;
pub mod boxed;
pub mod class;
mod config;
pub mod date;
pub mod function;
pub mod key;
pub mod object;
pub mod primitive;
pub mod promise;
pub mod regexp;
pub mod symbol;

pub const INDENT: &str = "  ";
pub const NEWLINE: &str = "\n";

/// Formats a [JavaScript Value](Value) as a string with the given [configuration](Config).
pub fn format_value(cx: &Context, cfg: Config, value: &Value) -> String {
	if !value.handle().is_object() {
		format_primitive(cx, cfg, value)
	} else {
		format_object(cx, cfg, value.to_object(cx))
	}
}
