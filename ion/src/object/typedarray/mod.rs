/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use mozjs::typedarray::{ArrayBufferU8, ClampedU8, Float32, Float64, Int16, Int32, Int8, Uint16, Uint32, Uint8};
use mozjs::typedarray as jsta;

pub use buffer::*;
pub use view::*;

use crate::{Context, Value};
use crate::conversions::{IntoValue, ToValue};

mod buffer;
mod view;

pub struct ArrayBufferWrapper {
	buf: Box<[<ArrayBufferU8 as jsta::TypedArrayElement>::Element]>,
}

impl ArrayBufferWrapper {
	pub fn into_array_buffer(self, cx: &Context) -> Option<ArrayBuffer> {
		ArrayBuffer::from_boxed_slice(cx, self.buf)
	}
}

impl<B: Into<Box<[<ArrayBufferU8 as jsta::TypedArrayElement>::Element]>>> From<B> for ArrayBufferWrapper {
	fn from(buffer: B) -> ArrayBufferWrapper {
		ArrayBufferWrapper { buf: buffer.into() }
	}
}

impl Deref for ArrayBufferWrapper {
	type Target = Box<[<ArrayBufferU8 as jsta::TypedArrayElement>::Element]>;

	fn deref(&self) -> &Self::Target {
		&self.buf
	}
}

impl<'cx> IntoValue<'cx> for ArrayBufferWrapper {
	fn into_value(self: Box<Self>, cx: &'cx Context, value: &mut Value) {
		if let Some(buffer) = self.into_array_buffer(cx) {
			buffer.to_value(cx, value);
		}
	}
}

macro_rules! impl_typedarray_wrapper {
	($(($typedarray:ident, $element:ty)$(,)?)*) => {
		$(
			pub struct $typedarray {
				buf: Box<[<$element as jsta::TypedArrayElement>::Element]>,
			}

			impl $typedarray {
				pub fn into_typed_array(self, cx: &Context) -> Option<TypedArray<$element>> {
					TypedArray::from_boxed_slice(cx, self.buf)
				}
			}

			impl<B: Into<Box<[<$element as jsta::TypedArrayElement>::Element]>>> From<B> for $typedarray {
				fn from(buffer: B) -> $typedarray {
					$typedarray { buf: buffer.into() }
				}
			}

			impl Deref for $typedarray {
				type Target = Box<[<$element as jsta::TypedArrayElement>::Element]>;

				fn deref(&self) -> &Self::Target {
					&self.buf
				}
			}

			impl<'cx> IntoValue<'cx> for $typedarray {
				fn into_value(self: Box<Self>, cx: &'cx Context, value: &mut Value) {
					if let Some(array) =  self.into_typed_array(cx) {
						array.to_value(cx, value);
					}
				}
			}
		)*
	};
}

impl_typedarray_wrapper!(
	(Uint8ArrayWrapper, Uint8),
	(Uint16ArrayWrapper, Uint16),
	(Uint32ArrayWrapper, Uint32),
	(Int8ArrayWrapper, Int8),
	(Int16ArrayWrapper, Int16),
	(Int32ArrayWrapper, Int32),
	(Float32ArrayWrapper, Float32),
	(Float64ArrayWrapper, Float64),
	(ClampedUint8ArrayWrapper, ClampedU8),
);
