/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter, Write};

use colored::Colorize;

use crate::format::{Config, INDENT, NEWLINE};
use crate::typedarray::buffer::ArrayBuffer;

pub fn format_array_buffer<'cx>(cfg: Config, buffer: &'cx ArrayBuffer<'cx>) -> ArrayBufferDisplay<'cx> {
	ArrayBufferDisplay { buffer, cfg }
}

pub struct ArrayBufferDisplay<'cx> {
	buffer: &'cx ArrayBuffer<'cx>,
	cfg: Config,
}

impl Display for ArrayBufferDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let colour = self.cfg.colours.object;
		write!(f, "{}", "ArrayBuffer {".color(colour))?;

		let bytes = unsafe { self.buffer.as_slice() };

		let indent = INDENT.repeat((self.cfg.indentation + self.cfg.depth + 1) as usize);
		if bytes.len() < 8 {
			f.write_char(' ')?;
		} else {
			f.write_str(NEWLINE)?;
			f.write_str(&indent)?;
		}

		write!(
			f,
			"{}{} ",
			"[Uint8Contents]".color(self.cfg.colours.symbol),
			":".color(colour)
		)?;
		f.write_str("<")?;

		for (i, byte) in bytes.iter().enumerate() {
			write!(f, "{:02x}", *byte)?;

			if i != bytes.len() - 1 {
				f.write_char(' ')?;
			}
		}

		f.write_char('>')?;
		write!(f, "{}", ",".color(colour))?;
		if bytes.len() < 8 {
			f.write_char(' ')?;
		} else {
			f.write_str(NEWLINE)?;
			f.write_str(&indent)?;
		}

		write!(f, "byteLength: {}", bytes.len())?;
		if bytes.len() < 8 {
			f.write_char(' ')?;
		} else {
			f.write_str(NEWLINE)?;
		}

		write!(f, "{}", "}".color(colour))?;

		Ok(())
	}
}
