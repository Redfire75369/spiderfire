/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::CStr;
use std::fmt;
use std::fmt::{Display, Formatter};

use colored::Colorize;
use mozjs::rust::get_object_class;

use crate::{Context, Object};
use crate::format::Config;
use crate::format::object::format_plain_object;

/// Formats a [JavaScript Object](Object), along with the name of its constructor, with the given [configuration](Config).
pub fn format_class_object<'cx>(cx: &'cx Context, cfg: Config, object: &'cx Object<'cx>) -> ClassObjectDisplay<'cx> {
	ClassObjectDisplay { cx, object, cfg }
}

#[must_use]
pub struct ClassObjectDisplay<'cx> {
	cx: &'cx Context,
	object: &'cx Object<'cx>,
	cfg: Config,
}

impl Display for ClassObjectDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let class = unsafe { get_object_class(self.object.handle().get()) };
		let name = unsafe { CStr::from_ptr((*class).name) }.to_str().unwrap();
		name.color(self.cfg.colours.object).fmt(f)?;
		f.write_str(" ")?;
		format_plain_object(self.cx, self.cfg, self.object).fmt(f)
	}
}
