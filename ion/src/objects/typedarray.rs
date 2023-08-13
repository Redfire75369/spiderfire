/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use mozjs::typedarray::CreateWith;

use crate::{Context, Error, Object, Value};
use crate::conversions::ToValue;
use crate::exception::ThrowException;

macro_rules! impl_typedarray_wrapper {
	($typedarray:ident, $ty:ty) => {
		pub struct $typedarray {
			pub buf: Vec<$ty>,
		}

		impl Deref for $typedarray {
			type Target = Vec<$ty>;

			fn deref(&self) -> &Self::Target {
				&self.buf
			}
		}

		impl<'cx> ToValue<'cx> for $typedarray {
			unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
				let mut typedarray = Object::new(cx);
				if mozjs::typedarray::$typedarray::create(**cx, CreateWith::Slice(self.buf.as_slice()), typedarray.handle_mut()).is_ok() {
					typedarray.to_value(cx, value);
				} else {
					Error::new(concat!("Failed to create", stringify!($typedarray)), None).throw(cx)
				}
			}
		}
	};
}

impl_typedarray_wrapper!(Uint8Array, u8);
impl_typedarray_wrapper!(Uint16Array, u16);
impl_typedarray_wrapper!(Uint32Array, u32);
impl_typedarray_wrapper!(Int8Array, i8);
impl_typedarray_wrapper!(Int16Array, i16);
impl_typedarray_wrapper!(Int32Array, i32);
impl_typedarray_wrapper!(Float32Array, f32);
impl_typedarray_wrapper!(Float64Array, f64);
impl_typedarray_wrapper!(Uint8ClampedArray, u8);
impl_typedarray_wrapper!(ArrayBuffer, u8);
