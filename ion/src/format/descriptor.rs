/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Display, Formatter};
use colored::Colorize;
use crate::format::{Config, format_value};
use crate::{Context, PropertyDescriptor};

/// Formats a [descriptor](PropertyDescriptor) with the given [configuration](Config).
pub fn format_descriptor<'cx>(
	cx: &'cx Context, cfg: Config, desc: &'cx PropertyDescriptor<'cx>,
) -> DescriptorDisplay<'cx> {
	DescriptorDisplay { cx, cfg, desc }
}

pub struct DescriptorDisplay<'cx> {
	cx: &'cx Context,
	cfg: Config,
	desc: &'cx PropertyDescriptor<'cx>,
}

impl Display for DescriptorDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match (self.desc.getter(self.cx), self.desc.setter(self.cx)) {
			(Some(_), Some(_)) => "[Getter/Setter]".color(self.cfg.colours.function).fmt(f),
			(Some(_), None) => "[Getter]".color(self.cfg.colours.function).fmt(f),
			(None, Some(_)) => "[Setter]".color(self.cfg.colours.function).fmt(f),
			(None, None) => match self.desc.value(self.cx) {
				Some(value) => format_value(self.cx, self.cfg.depth(self.cfg.depth + 1).quoted(true), &value).fmt(f),
				None => unreachable!(),
			},
		}
	}
}
