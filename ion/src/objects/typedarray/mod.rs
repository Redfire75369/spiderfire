/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;
use std::ptr;

use mozjs::jsapi::JSObject;
use mozjs::typedarray::CreateWith;

use crate::{Context, Error, Object, Result, Value};
use crate::conversions::ToValue;

pub mod buffer;

macro_rules! impl_typedarray_wrapper {
	($typedarray:ident, $ty:ty) => {
		pub struct $typedarray {
			buf: Box<[$ty]>,
		}

		impl $typedarray {
			pub fn to_object(&self, cx: &Context) -> Result<Object> {
				rooted!(in(cx.as_ptr()) let mut typed_array: *mut JSObject = ptr::null_mut());
				if unsafe {
					mozjs::typedarray::$typedarray::create(
						cx.as_ptr(),
						CreateWith::Slice(&self.buf),
						typed_array.handle_mut(),
					)
					.is_ok()
				} {
					Ok(Object::from(cx.root(typed_array.get())))
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

		impl ToValue for $typedarray {
			fn to_value(&self, cx: &Context) -> Result<Value> {
				self.to_object(cx).and_then(|obj| obj.to_value(cx))
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
