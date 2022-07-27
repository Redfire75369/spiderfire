/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::conversions::ConversionResult::Success;
use mozjs::conversions::FromJSValConvertible;
use mozjs::jsval::JSVal;

use crate::Context;

/// Converts a [JSVal] to a Rust type.
/// Returns [None] if the conversion fails.
pub fn from_value<T: FromJSValConvertible>(cx: Context, value: JSVal, config: T::Config) -> Option<T> {
	rooted!(in(cx) let rooted_val = value);
	if let Ok(Success(v)) = unsafe { T::from_jsval(cx, rooted_val.handle(), config) } {
		Some(v)
	} else {
		None
	}
}
