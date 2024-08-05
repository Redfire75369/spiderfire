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
	JSStructuredCloneCallbacks, StructuredCloneScope,
};

use ion::{Context, Exception, Object, ResultExc, Value};
use ion::conversions::ToValue;
use ion::flags::PropertyFlags;
use ion::function::Opt;

static STRUCTURED_CLONE_CALLBACKS: JSStructuredCloneCallbacks = JSStructuredCloneCallbacks {
	read: None,
	write: None,
	reportError: None,
	readTransfer: None,
	writeTransfer: None,
	freeTransfer: None,
	canTransfer: None,
	sabCloned: None,
};

pub fn write(
	cx: &Context, data: Value, transfer: Option<Vec<Object>>, scope: StructuredCloneScope,
) -> ResultExc<Vec<u8>> {
	let transfer = transfer.map_or_else(|| Value::undefined(cx), |t| t.as_value(cx));

	unsafe {
		let buffer = NewJSAutoStructuredCloneBuffer(scope, &STRUCTURED_CLONE_CALLBACKS);
		let scdata = &mut (*buffer).data_;

		let policy = CloneDataPolicy {
			allowIntraClusterClonableSharedObjects_: false,
			allowSharedMemoryObjects_: true,
		};

		let res = JS_WriteStructuredClone(
			cx.as_ptr(),
			data.handle().into(),
			scdata,
			scope,
			&policy,
			&STRUCTURED_CLONE_CALLBACKS,
			ptr::null_mut(),
			transfer.handle().into(),
		);

		if !res {
			return Err(Exception::new(cx)?.unwrap());
		}

		let len = GetLengthOfJSStructuredCloneData(scdata);
		let mut data = Vec::with_capacity(len);
		CopyJSStructuredCloneData(scdata, data.as_mut_ptr());
		data.set_len(len);

		DeleteJSAutoStructuredCloneBuffer(buffer);

		Ok(data)
	}
}

pub fn read(cx: &Context, data: Vec<u8>, scope: StructuredCloneScope) -> ResultExc<Value> {
	let mut rval = Value::undefined(cx);
	unsafe {
		let buffer = NewJSAutoStructuredCloneBuffer(scope, &STRUCTURED_CLONE_CALLBACKS);
		let scdata = &mut (*buffer).data_;

		WriteBytesToJSStructuredCloneData(data.as_ptr(), data.len(), scdata);

		let policy = CloneDataPolicy {
			allowIntraClusterClonableSharedObjects_: false,
			allowSharedMemoryObjects_: true,
		};

		let res = JS_ReadStructuredClone(
			cx.as_ptr(),
			scdata,
			JS_STRUCTURED_CLONE_VERSION,
			scope,
			rval.handle_mut().into(),
			&policy,
			&STRUCTURED_CLONE_CALLBACKS,
			ptr::null_mut(),
		);

		DeleteJSAutoStructuredCloneBuffer(buffer);

		if !res {
			Err(Exception::new(cx)?.unwrap())
		} else {
			Ok(rval)
		}
	}
}

#[derive(FromValue)]
struct StructuredCloneOptions<'cx> {
	transfer: Vec<Object<'cx>>,
}

#[js_fn]
fn structured_clone<'cx>(
	cx: &'cx Context, data: Value<'cx>, Opt(options): Opt<StructuredCloneOptions<'cx>>,
) -> ResultExc<Value<'cx>> {
	let transfer = options.map(|o| o.transfer);
	let data = write(cx, data, transfer, StructuredCloneScope::DifferentProcess)?;
	read(cx, data, StructuredCloneScope::DifferentProcess)
}

pub fn define(cx: &Context, global: &Object) -> bool {
	!global
		.define_method(
			cx,
			"structuredClone",
			structured_clone,
			1,
			PropertyFlags::CONSTANT_ENUMERATED,
		)
		.handle()
		.is_null()
}
