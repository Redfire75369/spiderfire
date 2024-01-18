/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter, Write};

use colored::Colorize;
use mozjs::jsapi::JSProtoKey;

use crate::{Array, Context};
use crate::format::{indent_str, NEWLINE};
use crate::format::Config;
use crate::format::descriptor::format_descriptor;
use crate::format::object::{write_prefix, write_remaining};

/// Formats an [JavaScript Array](Array) using the given [configuration](Config).
pub fn format_array<'cx>(cx: &'cx Context, cfg: Config, array: &'cx Array<'cx>) -> ArrayDisplay<'cx> {
	ArrayDisplay { cx, array, cfg }
}

#[must_use]
pub struct ArrayDisplay<'cx> {
	cx: &'cx Context,
	array: &'cx Array<'cx>,
	cfg: Config,
}

impl Display for ArrayDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let colour = self.cfg.colours.array;

		write_prefix(
			f,
			self.cx,
			self.cfg,
			self.array.as_object(),
			"Array",
			JSProtoKey::JSProto_Array,
		)?;

		if self.cfg.depth < 5 {
			let length = self.array.len(self.cx);

			if length == 0 {
				"[]".color(colour).fmt(f)
			} else {
				"[".color(colour).fmt(f)?;

				let (remaining, inner) = if self.cfg.multiline {
					f.write_str(NEWLINE)?;
					let len = length.clamp(0, 100);

					let inner = indent_str((self.cfg.indentation + self.cfg.depth + 1) as usize);

					for index in 0..len {
						inner.fmt(f)?;
						let desc = self.array.get_descriptor(self.cx, index)?.unwrap();
						format_descriptor(self.cx, self.cfg, &desc, Some(self.array.as_object())).fmt(f)?;
						",".color(colour).fmt(f)?;
						f.write_str(NEWLINE)?;
					}

					(length - len, Some(inner))
				} else {
					f.write_char(' ')?;
					let len = length.clamp(0, 3);

					for index in 0..len {
						let desc = self.array.get_descriptor(self.cx, index)?.unwrap();
						format_descriptor(self.cx, self.cfg, &desc, Some(self.array.as_object())).fmt(f)?;

						if index != len - 1 {
							",".color(colour).fmt(f)?;
							f.write_char(' ')?;
						}
					}

					(length - len, None)
				};

				write_remaining(f, remaining as usize, inner.as_deref(), colour)?;

				if self.cfg.multiline {
					indent_str((self.cfg.indentation + self.cfg.depth) as usize).fmt(f)?;
				}

				"]".color(colour).fmt(f)
			}
		} else {
			"[Array]".color(colour).fmt(f)
		}
	}
}
