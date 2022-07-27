/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::{ESClass, JSObject, Unbox};

use crate::{Context, Object};
use crate::format::Config;
use crate::format::primitive::format_primitive;

/// Formats a boxed primitive ([Object]) as a [String] using the given [Config].
/// The supported boxed types are `Boolean`, `Number`, `String` and `BigInt`.
///
/// ### Unimplemented
/// - Support for `BigInt`
pub fn format_boxed(cx: Context, cfg: Config, object: *mut JSObject, class: ESClass) -> String {
	rooted!(in(cx) let robj = object);
	rooted!(in(cx) let mut unboxed = Object::new(cx).to_value());

	unsafe {
		if Unbox(cx, robj.handle().into(), unboxed.handle_mut().into()) {
			use ESClass::*;
			match class {
				Boolean | Number | String => format_primitive(cx, cfg, unboxed.get()),
				BigInt => format!("Unimplemented Formatting: {}", "BigInt"),
				_ => unreachable!(),
			}
		} else {
			String::from("Boxed Value")
		}
	}
}
