/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::{Context, Result, Value};
use crate::conversions::ToValue;

pub type BoxedIntoValue = Box<dyn IntoValue>;

/// Represents types that can be converted to JavaScript [Values](Value) with ownership.
/// Primarily used with dynamic dispatch and [`BoxedIntoValue`](BoxedIntoValue).
pub trait IntoValue {
	/// Converts `self` into a [`Value`](Value) and stores it in `value`.
	fn into_value(self: Box<Self>, cx: &Context) -> Result<Value>;
}

impl<T: ToValue> IntoValue for T {
	fn into_value(self: Box<Self>, cx: &Context) -> Result<Value> {
		self.to_value(cx)
	}
}
