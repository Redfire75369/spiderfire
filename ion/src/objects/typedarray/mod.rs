/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use mozjs::typedarray::CreateWith;

use crate::{Context, Error, Object, Result, Value};
use crate::conversions::ToValue;
use crate::exception::ThrowException;

pub mod buffer;
pub mod view;

macro_rules! impl_typedarray_wrapper {
	($typedarray:ident, $ty:ty) => {
		pub struct $typedarray {
			buf: Box<[$ty]>,
		}

		impl $typedarray {
			pub fn to_object<'cx>(&self, cx: &'cx Context) -> Result<Object<'cx>> {
				let mut typed_array = Object::new(cx);
				if unsafe {
					mozjs::typedarray::$typedarray::create(
						cx.as_ptr(),
						CreateWith::Slice(&self.buf),
						typed_array.handle_mut(),
					)
					.is_ok()
				} {
					Ok(typed_array)
				} else {
					Err(Error::new(
						concat!("Failed to create", stringify!($typedarray)),
						None,
					))
				}
			}
		}

		impl<B: Into<Box<[$ty]>>> From<B> for $typedarray {
			fn from(buffer: B) -> $typedarray {
				$typedarray { buf: buffer.into() }
			}
		}

		impl Deref for $typedarray {
			type Target = Box<[$ty]>;

			fn deref(&self) -> &Self::Target {
				&self.buf
			}
		}

		impl<'cx> ToValue<'cx> for $typedarray {
			fn to_value(&self, cx: &'cx Context, value: &mut Value) {
				match self.to_object(cx) {
					Ok(typed_array) => typed_array.to_value(cx, value),
					Err(error) => error.throw(cx),
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
