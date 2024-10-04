/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::Any;
use std::ffi::c_void;
use std::ptr;

use mozjs::glue::{
	CopyJSStructuredCloneData, DeleteJSAutoStructuredCloneBuffer, GetLengthOfJSStructuredCloneData,
	NewJSAutoStructuredCloneBuffer, WriteBytesToJSStructuredCloneData,
};
use mozjs::jsapi::{
	CloneDataPolicy, JSAutoStructuredCloneBuffer, JSStructuredCloneCallbacks, JSStructuredCloneReader,
	JSStructuredCloneWriter, JS_ReadStructuredClone, JS_ReadUint32Pair, JS_WriteStructuredClone, JS_WriteUint32Pair,
	StructuredCloneScope, JS_STRUCTURED_CLONE_VERSION,
};

use crate::conversions::ToValue;
use crate::{Context, Exception, Object, ResultExc, Value};

pub struct StructuredCloneBuffer {
	buf: *mut JSAutoStructuredCloneBuffer,
	scope: StructuredCloneScope,
	callbacks: &'static JSStructuredCloneCallbacks,
	data: Box<Option<Box<dyn Any>>>,
}

impl StructuredCloneBuffer {
	pub fn new(
		scope: StructuredCloneScope, callbacks: &'static JSStructuredCloneCallbacks, data: Option<Box<dyn Any>>,
	) -> StructuredCloneBuffer {
		StructuredCloneBuffer {
			buf: unsafe { NewJSAutoStructuredCloneBuffer(scope, callbacks) },
			scope,
			callbacks,
			data: Box::new(data),
		}
	}

	pub fn write(
		&mut self, cx: &Context, data: &Value, transfer: Option<Vec<Object>>, policy: &CloneDataPolicy,
	) -> ResultExc<()> {
		let transfer = transfer.map_or_else(|| Value::undefined(cx), |t| t.as_value(cx));

		unsafe {
			let scdata = &mut (*self.buf).data_;

			let res = JS_WriteStructuredClone(
				cx.as_ptr(),
				data.handle().into(),
				scdata,
				self.scope,
				policy,
				self.callbacks,
				self.data_ptr(),
				transfer.handle().into(),
			);

			if res {
				Ok(())
			} else {
				Err(Exception::new(cx)?.unwrap())
			}
		}
	}

	pub fn read<'cx>(&self, cx: &'cx Context, policy: &CloneDataPolicy) -> ResultExc<Value<'cx>> {
		let mut rval = Value::undefined(cx);
		unsafe {
			let scdata = &mut (*self.buf).data_;

			let res = JS_ReadStructuredClone(
				cx.as_ptr(),
				scdata,
				JS_STRUCTURED_CLONE_VERSION,
				self.scope,
				rval.handle_mut().into(),
				policy,
				self.callbacks,
				self.data_ptr(),
			);

			if res {
				Ok(rval)
			} else {
				Err(Exception::new(cx)?.unwrap())
			}
		}
	}

	/// Converts the buffer into bytes. If the buffer contains pointers
	/// (contains transferable objects and scope is JSStructuredCloneScope::SameProcess),
	/// the Buffer must remain alive for the transferable objects to be valid.
	pub unsafe fn to_vec(&self) -> Vec<u8> {
		unsafe {
			let scdata = &mut (*self.buf).data_;

			let len = GetLengthOfJSStructuredCloneData(scdata);
			let mut data = Vec::with_capacity(len);
			CopyJSStructuredCloneData(scdata, data.as_mut_ptr());
			data.set_len(len);

			data
		}
	}

	/// Reads the data into the buffer.
	/// The data must not contain pointers (see [StructuredCloneBuffer::to_vec]).
	pub unsafe fn write_from_bytes(&self, data: &[u8]) {
		let scdata = &mut (*self.buf).data_;
		unsafe {
			WriteBytesToJSStructuredCloneData(data.as_ptr(), data.len(), scdata);
		}
	}

	fn data_ptr(&self) -> *mut c_void {
		ptr::from_ref(&*self.data).cast::<c_void>().cast_mut()
	}
}

impl Drop for StructuredCloneBuffer {
	fn drop(&mut self) {
		unsafe {
			DeleteJSAutoStructuredCloneBuffer(self.buf);
		}
	}
}

pub unsafe fn read_uint64(r: *mut JSStructuredCloneReader) -> Option<u64> {
	let mut high = 0;
	let mut low = 0;
	let res = unsafe { JS_ReadUint32Pair(r, &mut high, &mut low) };
	res.then_some(((high as u64) << 32) | (low as u64))
}

pub unsafe fn write_uint64(w: *mut JSStructuredCloneWriter, data: u64) -> bool {
	JS_WriteUint32Pair(w, (data >> 32) as u32, (data & 0xFFFFFFFF) as u32)
}
