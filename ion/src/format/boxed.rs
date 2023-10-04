/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::{Context, Object};
use crate::format::Config;
use crate::format::primitive::format_primitive;

/// Formats a boxed primitive ([Object]) as a string using the given [configuration](Config).
/// The supported boxed types are `Boolean`, `Number`, `String` and `BigInt`.
pub fn format_boxed(cx: &Context, cfg: Config, object: &Object) -> String {
	if let Some(primitive) = object.unbox_primitive(cx) {
		format_primitive(cx, cfg, &primitive)
	} else {
		String::from("(boxed value)")
	}
}
