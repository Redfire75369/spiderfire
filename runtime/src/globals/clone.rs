/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::c_void;
use std::ptr;
use bytes::{Bytes, BytesMut};
use mozjs::jsapi::{
	CloneDataPolicy, Handle, JSContext, JSObject, JSStructuredCloneCallbacks, JSStructuredCloneReader,
	JSStructuredCloneWriter, JS_ReadBytes, JS_ReadString, JS_ReadUint32Pair, JS_WriteBytes, JS_WriteString,
	JS_WriteUint32Pair, StructuredCloneScope,
};

use ion::{ClassDefinition, Context, Local, Object, ResultExc, Value};
use ion::class::Reflector;
use ion::clone::StructuredCloneBuffer;
use ion::flags::PropertyFlags;
use ion::function::Opt;
use crate::globals::file::Blob;

#[repr(u32)]
enum StructuredCloneTags {
	Min = 0xFFFF8000,
	Blob = 0xFFFF8001,
	Max = 0xFFFFFFFF,
}

unsafe extern "C" fn read_callback(
	cx: *mut JSContext, r: *mut JSStructuredCloneReader, _: *const CloneDataPolicy, tag: u32, _data: u32,
	_private: *mut c_void,
) -> *mut JSObject {
	assert!(
		tag > StructuredCloneTags::Min as u32,
		"Expected Tag Below StructuredCloneTags::Min"
	);
	assert!(
		tag < StructuredCloneTags::Max as u32,
		"Expected Tag Below StructuredCloneTags::Max"
	);

	let cx = unsafe { &Context::new_unchecked(cx) };
	if tag == StructuredCloneTags::Blob as u32 {
		let mut len_high = 0;
		let mut len_low = 0;
		let mut bytes;
		let mut kind = ion::String::new(cx);

		unsafe {
			JS_ReadUint32Pair(r, &mut len_high, &mut len_low);
			let len = ((len_high as usize) << 32) | (len_low as usize);
			bytes = BytesMut::with_capacity(len);
			JS_ReadBytes(r, bytes.as_mut_ptr().cast(), len);
			bytes.set_len(len);
			JS_ReadString(r, kind.handle_mut().into());
		}

		Blob::new_object(
			cx,
			Box::new(Blob {
				reflector: Reflector::default(),
				bytes: bytes.freeze(),
				kind: Some(kind.to_owned(cx).unwrap()),
			}),
		)
	} else {
		ptr::null_mut()
	}
}

unsafe extern "C" fn write_callback(
	cx: *mut JSContext, w: *mut JSStructuredCloneWriter, obj: Handle<*mut JSObject>, _same_process_scope: *mut bool,
	_private: *mut c_void,
) -> bool {
	let cx = unsafe { &Context::new_unchecked(cx) };
	let object = Object::from(unsafe { Local::from_raw_handle(obj) });

	if let Ok(blob) = Blob::get_private(cx, &object) {
		let kind = ion::String::copy_from_str(cx, blob.kind.as_deref().unwrap_or("")).unwrap();

		unsafe {
			JS_WriteUint32Pair(w, StructuredCloneTags::Blob as u32, 0);
			JS_WriteUint32Pair(
				w,
				(blob.bytes.len() >> 32) as u32,
				(blob.bytes.len() & 0xFFFFFFFF) as u32,
			);
			JS_WriteBytes(w, blob.bytes.as_ptr().cast(), blob.bytes.len());
			JS_WriteString(w, kind.handle().into());
		}
	}

	true
}

pub static STRUCTURED_CLONE_CALLBACKS: JSStructuredCloneCallbacks = JSStructuredCloneCallbacks {
	read: Some(read_callback),
	write: Some(write_callback),
	reportError: None,
	readTransfer: None,
	writeTransfer: None,
	freeTransfer: None,
	canTransfer: None,
	sabCloned: None,
};

#[derive(FromValue)]
struct StructuredCloneOptions<'cx> {
	transfer: Vec<Object<'cx>>,
}

#[js_fn]
fn structured_clone<'cx>(
	cx: &'cx Context, data: Value<'cx>, Opt(options): Opt<StructuredCloneOptions<'cx>>,
) -> ResultExc<Value<'cx>> {
	let transfer = options.map(|o| o.transfer);
	let policy = CloneDataPolicy {
		allowIntraClusterClonableSharedObjects_: false,
		allowSharedMemoryObjects_: true,
	};

	let mut buffer = StructuredCloneBuffer::new(StructuredCloneScope::SameProcess, &STRUCTURED_CLONE_CALLBACKS);
	buffer.write(cx, &data, transfer, &policy)?;
	buffer.read(cx, &policy)
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
