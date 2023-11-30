/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{ptr, slice};
use std::ffi::c_void;
use std::ops::{Deref, DerefMut};

use mozjs::jsapi::{
	ArrayBufferClone, ArrayBufferCopyData, DetachArrayBuffer, GetArrayBufferLengthAndData, Heap, IsArrayBufferObject,
	IsDetachedArrayBufferObject, JS_GetTypedArraySharedness, JSObject, NewArrayBufferWithContents,
	NewExternalArrayBuffer, StealArrayBufferContents,
};
use mozjs::typedarray::CreateWith;

use crate::{Context, Error, ErrorKind, Result, Root};
use crate::utils::BoxExt;

pub struct ArrayBuffer {
	buffer: Root<Box<Heap<*mut JSObject>>>,
}

impl ArrayBuffer {
	fn create_with(cx: &Context, with: CreateWith<u8>) -> Option<ArrayBuffer> {
		rooted!(in(cx.as_ptr()) let mut buffer: *mut JSObject = ptr::null_mut());
		unsafe { mozjs::typedarray::ArrayBuffer::create(cx.as_ptr(), with, buffer.handle_mut()).ok()? };
		Some(ArrayBuffer { buffer: cx.root(buffer.get()) })
	}

	/// Creates a new [ArrayBuffer] with the given length.
	pub fn new(cx: &Context, len: usize) -> Option<ArrayBuffer> {
		ArrayBuffer::create_with(cx, CreateWith::Length(len))
	}

	/// Creates a new [ArrayBuffer] by copying the contents of the given slice.
	pub fn copy_from_bytes(cx: &Context, bytes: &[u8]) -> Option<ArrayBuffer> {
		ArrayBuffer::create_with(cx, CreateWith::Slice(bytes))
	}

	/// Creates a new [ArrayBuffer] by transferring ownership of the bytes to the JS runtime.
	pub fn from_vec(cx: &Context, bytes: Vec<u8>) -> Option<ArrayBuffer> {
		ArrayBuffer::from_boxed_slice(cx, bytes.into_boxed_slice())
	}

	/// Creates a new [ArrayBuffer] by transferring ownership of the bytes to the JS runtime.
	pub fn from_boxed_slice(cx: &Context, bytes: Box<[u8]>) -> Option<ArrayBuffer> {
		unsafe extern "C" fn free_external_array_buffer(contents: *mut c_void, data: *mut c_void) {
			let _ = unsafe { Box::from_raw_parts(contents.cast::<u8>(), data as usize) };
		}

		let (ptr, len) = unsafe { Box::into_raw_parts(bytes) };
		let buffer = unsafe {
			NewExternalArrayBuffer(
				cx.as_ptr(),
				len,
				ptr.cast(),
				Some(free_external_array_buffer),
				len as *mut c_void,
			)
		};

		if buffer.is_null() {
			None
		} else {
			Some(ArrayBuffer { buffer: cx.root_object(buffer) })
		}
	}

	pub fn from(object: Root<Box<Heap<*mut JSObject>>>) -> Option<ArrayBuffer> {
		if ArrayBuffer::is_array_buffer(object.get()) {
			Some(ArrayBuffer { buffer: object })
		} else {
			None
		}
	}

	pub unsafe fn from_unchecked(object: Root<Box<Heap<*mut JSObject>>>) -> ArrayBuffer {
		ArrayBuffer { buffer: object }
	}

	/// Returns a pointer and length to the contents of the [ArrayBuffer].
	///
	/// The pointer may be invalidated if the [ArrayBuffer] is detached.
	pub fn data(&self) -> (*mut u8, usize) {
		let mut len = 0;
		let mut shared = false;
		let mut data = ptr::null_mut();
		unsafe { GetArrayBufferLengthAndData(self.get(), &mut len, &mut shared, &mut data) };
		(data, len)
	}

	/// Returns a slice to the contents of the [ArrayBuffer].
	///
	/// The slice may be invalidated if the [ArrayBuffer] is detached.
	pub unsafe fn as_slice(&self) -> &[u8] {
		let (ptr, len) = self.data();
		unsafe { slice::from_raw_parts(ptr, len) }
	}

	/// Returns a mutable slice to the contents of the [ArrayBuffer].
	///
	/// The slice may be invalidated if the [ArrayBuffer] is detached.
	pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
		let (ptr, len) = self.data();
		unsafe { slice::from_raw_parts_mut(ptr, len) }
	}

	/// Clones an [ArrayBuffer].
	pub fn clone(&self, cx: &Context, offset: usize, len: usize) -> Option<ArrayBuffer> {
		let buffer = unsafe { ArrayBufferClone(cx.as_ptr(), self.handle().into(), offset, len) };
		if buffer.is_null() {
			None
		} else {
			Some(ArrayBuffer { buffer: cx.root_object(buffer) })
		}
	}

	/// Copies data from one [ArrayBuffer] to another.
	/// Returns `false` if the sizes do not match.
	pub fn copy_data_to(
		&self, cx: &Context, to: &mut ArrayBuffer, from_index: usize, to_index: usize, count: usize,
	) -> bool {
		unsafe {
			ArrayBufferCopyData(
				cx.as_ptr(),
				to.handle().into(),
				to_index,
				self.handle().into(),
				from_index,
				count,
			)
		}
	}

	pub fn detach(&mut self, cx: &Context) -> bool {
		unsafe { DetachArrayBuffer(cx.as_ptr(), self.handle().into()) }
	}

	pub fn transfer(&mut self, cx: &Context) -> Result<ArrayBuffer> {
		let len = self.data().1;
		let data = unsafe { StealArrayBufferContents(cx.as_ptr(), self.handle().into()) };
		if data.is_null() {
			return Err(Error::new("ArrayBuffer transfer failed", ErrorKind::Normal));
		}
		let buffer = cx.root_object(unsafe { NewArrayBufferWithContents(cx.as_ptr(), len, data) });
		if buffer.handle().is_null() {
			return Err(Error::new("ArrayBuffer transfer failed", ErrorKind::Normal));
		}
		Ok(ArrayBuffer { buffer })
	}

	pub fn is_detached(&self) -> bool {
		unsafe { IsDetachedArrayBufferObject(self.get()) }
	}

	pub fn is_shared(&self) -> bool {
		unsafe { JS_GetTypedArraySharedness(self.get()) }
	}

	/// Checks if an object is an array buffer.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn is_array_buffer(object: *mut JSObject) -> bool {
		unsafe { IsArrayBufferObject(object) }
	}
}

impl Deref for ArrayBuffer {
	type Target = Root<Box<Heap<*mut JSObject>>>;

	fn deref(&self) -> &Self::Target {
		&self.buffer
	}
}

impl DerefMut for ArrayBuffer {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.buffer
	}
}
