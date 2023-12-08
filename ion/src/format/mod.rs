/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};

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
pub mod typedarray;

pub const INDENT: &str = "  ";
pub const NEWLINE: &str = "\n";

/// Formats a [JavaScript Value](Value) as a string with the given [configuration](Config).
pub fn format_value<'cx>(cx: &'cx Context, cfg: Config, value: &'cx Value<'cx>) -> ValueDisplay<'cx> {
	ValueDisplay { cx, value, cfg }
}

pub struct ValueDisplay<'cx> {
	cx: &'cx Context,
	value: &'cx Value<'cx>,
	cfg: Config,
}

impl Display for ValueDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		if self.value.handle().is_object() {
			write!(f, "{}", format_object(self.cx, self.cfg, self.value.to_object(self.cx)))
		} else {
			write!(f, "{}", format_primitive(self.cx, self.cfg, self.value))
		}
	}
}
