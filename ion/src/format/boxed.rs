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

/// Formats a boxed primitive ([Object]) using the given [configuration](Config).
/// The supported boxed types are `Boolean`, `Number`, `String` and `BigInt`.
pub fn format_boxed_primitive<'cx>(
	cx: &'cx Context, cfg: Config, object: &'cx Object<'cx>,
) -> BoxedPrimitiveDisplay<'cx> {
	BoxedPrimitiveDisplay { cx, object, cfg }
}

#[must_use]
pub struct BoxedPrimitiveDisplay<'cx> {
	cx: &'cx Context,
	object: &'cx Object<'cx>,
	cfg: Config,
}

impl Display for BoxedPrimitiveDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		if let Some(primitive) = self.object.unbox_primitive(self.cx) {
			format_primitive(self.cx, self.cfg, &primitive).fmt(f)
		} else {
			unreachable!("Object is not boxed")
		}
	}
}
