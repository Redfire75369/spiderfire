/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use mozjs::conversions::ToJSValConvertible;
use mozjs::jsval::ObjectOrNullValue;
use mozjs::rust::MutableHandleValue;
use mozjs::typedarray::CreateWith;

use crate::{Context, Error, Object};
use crate::error::ThrowException;

macro_rules! impl_typedarray_wrapper {
	($typedarray:ident, $ty:ty, $name:literal) => {
		pub struct $typedarray {
			pub buf: Vec<$ty>,
		}

		impl Deref for $typedarray {
			type Target = Vec<$ty>;

			fn deref(&self) -> &Self::Target {
				&self.buf
			}
		}

		impl DerefMut for $typedarray {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.buf
			}
		}

		impl ToJSValConvertible for $typedarray {
			unsafe fn to_jsval(&self, cx: Context, mut rval: MutableHandleValue) {
				rooted!(in(cx) let mut array = *Object::new(cx));
				if mozjs::typedarray::$typedarray::create(cx, CreateWith::Slice(self.buf.as_slice()), array.handle_mut()).is_ok() {
					rval.set(ObjectOrNullValue(array.get()));
				} else {
					Error::new(concat!("Failed to create", $name), None).throw(cx)
				}
			}
		}
	}
}

impl_typedarray_wrapper!(Uint8Array, u8, "Uint8Array");
impl_typedarray_wrapper!(Uint16Array, u16, "Uint16Array");
impl_typedarray_wrapper!(Uint32Array, u32, "Uint32Array");
impl_typedarray_wrapper!(Int8Array, i8, "Int8Array");
impl_typedarray_wrapper!(Int16Array, i16, "Int16Array");
impl_typedarray_wrapper!(Int32Array, i32, "Int32Array");
impl_typedarray_wrapper!(Float32Array, f32, "Float32Array");
impl_typedarray_wrapper!(Float64Array, f64, "Float64Array");
impl_typedarray_wrapper!(Uint8ClampedArray, u8, "Uint8ClampedArray");
impl_typedarray_wrapper!(ArrayBuffer, u8, "ArrayBuffer");
