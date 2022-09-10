/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::conversions::ToJSValConvertible;
use mozjs::rust::MutableHandleValue;

use crate::Context;

pub trait IntoJSVal {
	unsafe fn into_jsval(self: Box<Self>, cx: Context, rval: MutableHandleValue);
}

impl<T: ToJSValConvertible> IntoJSVal for T {
	unsafe fn into_jsval(self: Box<Self>, cx: Context, rval: MutableHandleValue) {
		self.to_jsval(cx, rval)
	}
}
