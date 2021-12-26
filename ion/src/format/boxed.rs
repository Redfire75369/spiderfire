/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::{ESClass, Unbox};

use crate::format::config::FormatConfig;
use crate::format::primitive::format_primitive;
use crate::IonContext;
use crate::objects::object::{IonObject, IonRawObject};

/// Formats a boxed object to a [String] using the given configuration options
/// Supported types are `Boolean`, `Number`, `String` and `BigInt`
pub fn format_boxed(cx: IonContext, cfg: FormatConfig, object: IonRawObject, class: ESClass) -> String {
	rooted!(in(cx) let robj = object);
	rooted!(in(cx) let mut unboxed = IonObject::new(cx).to_value());

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
