/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ptr;
use mozjs::glue::{
	CopyJSStructuredCloneData, DeleteJSAutoStructuredCloneBuffer, GetLengthOfJSStructuredCloneData,
	NewJSAutoStructuredCloneBuffer, WriteBytesToJSStructuredCloneData,
};
use mozjs::jsapi::{
	CloneDataPolicy, JS_ReadStructuredClone, JS_STRUCTURED_CLONE_VERSION, JS_WriteStructuredClone,
	JSAutoStructuredCloneBuffer, JSStructuredCloneCallbacks, StructuredCloneScope,
};
use crate::{Context, Exception, Object, ResultExc, Value};
use crate::conversions::ToValue;

pub struct StructuredCloneBuffer {
	buf: *mut JSAutoStructuredCloneBuffer,
	scope: StructuredCloneScope,
	callbacks: &'static JSStructuredCloneCallbacks,
}

impl StructuredCloneBuffer {
	pub fn new(scope: StructuredCloneScope, callbacks: &'static JSStructuredCloneCallbacks) -> StructuredCloneBuffer {
		StructuredCloneBuffer {
			buf: unsafe { NewJSAutoStructuredCloneBuffer(scope, callbacks) },
			scope,
			callbacks,
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
				ptr::null_mut(),
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
				ptr::null_mut(),
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
}

impl Drop for StructuredCloneBuffer {
	fn drop(&mut self) {
		unsafe {
			DeleteJSAutoStructuredCloneBuffer(self.buf);
		}
	}
}
