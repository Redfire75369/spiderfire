/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};

use crate::{Context, Object};
use crate::format::Config;
use crate::format::primitive::format_primitive;

/// Formats a boxed primitive ([Object]) as a string using the given [configuration](Config).
/// The supported boxed types are `Boolean`, `Number`, `String` and `BigInt`.
pub fn format_boxed<'cx>(cx: &'cx Context, cfg: Config, object: &'cx Object<'cx>) -> BoxedDisplay<'cx> {
	BoxedDisplay { cx, object, cfg }
}

pub struct BoxedDisplay<'cx> {
	cx: &'cx Context,
	object: &'cx Object<'cx>,
	cfg: Config,
}

impl Display for BoxedDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		if let Some(primitive) = self.object.unbox_primitive(self.cx) {
			write!(f, "{}", format_primitive(self.cx, self.cfg, &primitive))
		} else {
			unreachable!("Object is not boxed")
		}
	}
}
