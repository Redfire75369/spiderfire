/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::{Context, Value};
use crate::conversions::ToValue;

pub type BoxedIntoValue<'cx> = Box<dyn IntoValue<'cx>>;

pub trait IntoValue<'cx> {
	unsafe fn into_value(self: Box<Self>, cx: &'cx Context, value: &mut Value);
}

impl<'cx, T: ToValue<'cx>> IntoValue<'cx> for T {
	unsafe fn into_value(self: Box<Self>, cx: &'cx Context, value: &mut Value) {
		self.to_value(cx, value)
	}
}
