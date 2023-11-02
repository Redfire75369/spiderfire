/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::c_void;
use std::slice;

use byteorder::NativeEndian;
use mozjs::glue::{CreateJSExternalStringCallbacks, JSExternalStringCallbacksTraps};
use mozjs::jsapi::JS_NewExternalString;
use mozjs::jsapi::mozilla::MallocSizeOf;
use utf16string::WString;

use crate::{Context, String};

pub(crate) fn new_external_string(cx: &Context, str: WString<NativeEndian>) -> Result<String, WString<NativeEndian>> {
	let vec = str.into_bytes();
	let boxed = vec.into_boxed_slice();

	let (chars, len) = box_into_raw(boxed);

	unsafe {
		let callbacks = CreateJSExternalStringCallbacks(&EXTERNAL_STRING_CALLBACKS_TRAPS, len as *mut c_void);
		let jsstr = JS_NewExternalString(cx.as_ptr(), chars, len, callbacks);

		if !jsstr.is_null() {
			Ok(String::from(cx.root_string(jsstr)))
		} else {
			let slice = slice::from_raw_parts_mut(chars as *mut u8, len * 2);
			let boxed = Box::from_raw(slice);
			let vec = Vec::from(boxed);
			Err(WString::from_utf16_unchecked(vec))
		}
	}
}

extern "C" fn finalise_external_string(private_data: *const c_void, chars: *mut u16) {
	let _ = box_from_raw(chars, private_data);
}

extern "C" fn size_of_buffer(private_data: *const c_void, _: *const u16, _: MallocSizeOf) -> usize {
	private_data as usize
}

static EXTERNAL_STRING_CALLBACKS_TRAPS: JSExternalStringCallbacksTraps = JSExternalStringCallbacksTraps {
	finalize: Some(finalise_external_string),
	sizeOfBuffer: Some(size_of_buffer),
};

fn box_into_raw(boxed: Box<[u8]>) -> (*const u16, usize) {
	assert_eq!(boxed.len() % 2, 0);
	let len = boxed.len() / 2;
	let chars = Box::into_raw(boxed) as *const u16;
	(chars, len)
}

fn box_from_raw(chars: *mut u16, private_data: *const c_void) -> Box<[u8]> {
	let len = private_data as usize;
	unsafe {
		let slice = slice::from_raw_parts_mut(chars as *mut u8, len * 2);
		Box::from_raw(slice)
	}
}

#[cfg(test)]
mod tests {
	use std::ffi::c_void;

	use byteorder::NativeEndian;
	use utf16string::WString;

	use crate::string::external::{box_from_raw, box_into_raw};

	type NativeWString = WString<NativeEndian>;

	#[test]
	fn wstring_into_box() {
		let string = "S\u{500}t\u{1000}r\u{5000}i\u{10000}n\u{50000}g";

		let base = NativeWString::from(string);
		let boxed = NativeWString::from(string).into_bytes().into_boxed_slice();
		let (chars, len) = box_into_raw(boxed);

		let boxed = box_from_raw(chars as *mut u16, len as *mut c_void);
		let wstring = NativeWString::from_utf16(Vec::from(boxed)).unwrap();
		assert_eq!(base, wstring);
	}
}
