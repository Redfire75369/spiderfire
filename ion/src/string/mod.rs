/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{ptr, slice};
use std::ffi::c_void;
use std::ops::{Deref, DerefMut, Range};
use std::string::String as RustString;

use bytemuck::cast_slice;
use byteorder::NativeEndian;
use mozjs::glue::{CreateJSExternalStringCallbacks, JSExternalStringCallbacksTraps};
use mozjs::jsapi::{
	JS_CompareStrings, JS_ConcatStrings, JS_DeprecatedStringHasLatin1Chars, JS_GetEmptyString,
	JS_GetLatin1StringCharsAndLength, JS_GetStringCharAt, JS_GetTwoByteStringCharsAndLength, JS_NewDependentString,
	JS_NewExternalString, JS_NewUCStringCopyN, JS_StringIsLinear, JSString,
};
use mozjs::jsapi::mozilla::MallocSizeOf;
use utf16string::{WStr, WString};

use crate::{Context, Local};
use crate::string::byte::{ByteStr, Latin1};
use crate::utils::BoxExt;

pub mod byte;

#[derive(Copy, Clone, Debug)]
pub enum StringRef<'s> {
	Latin1(&'s ByteStr<Latin1>),
	Utf16(&'s WStr<NativeEndian>),
}

impl StringRef<'_> {
	pub fn is_empty(&self) -> bool {
		match self {
			StringRef::Latin1(b) => b.is_empty(),
			StringRef::Utf16(wstr) => wstr.is_empty(),
		}
	}

	pub fn len(&self) -> usize {
		match self {
			StringRef::Latin1(b) => b.len(),
			StringRef::Utf16(wstr) => wstr.len(),
		}
	}

	pub fn as_bytes(&self) -> &[u8] {
		match self {
			StringRef::Latin1(b) => b,
			StringRef::Utf16(wstr) => wstr.as_bytes(),
		}
	}
}

/// Represents a primitive string in the JS Runtime.
/// Strings in JS are immutable and are copied on modification, other than concatenating and slicing.
///
/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String) for more details.
#[derive(Debug)]
pub struct String<'s> {
	str: Local<'s, *mut JSString>,
}

impl<'s> String<'s> {
	/// Creates an empty [String].
	pub fn new(cx: &Context) -> String {
		String::from(cx.root_string(unsafe { JS_GetEmptyString(cx.as_ptr()) }))
	}

	/// Creates a new [String] with a given string, by copying it to the JS Runtime.
	pub fn copy_from_str<'cx>(cx: &'cx Context, string: &str) -> Option<String<'cx>> {
		let utf16: Vec<u16> = string.encode_utf16().collect();
		let jsstr = unsafe { JS_NewUCStringCopyN(cx.as_ptr(), utf16.as_ptr(), utf16.len()) };
		if jsstr.is_null() {
			None
		} else {
			Some(String::from(cx.root_string(jsstr)))
		}
	}

	/// Creates a new string by moving ownership of the UTF-16 string to the JS Runtime temporarily.
	/// Returns the string if the creation of the string in the runtime fails.
	pub fn from_wstring(cx: &Context, string: WString<NativeEndian>) -> Result<String, WString<NativeEndian>> {
		unsafe extern "C" fn finalise_external_string(data: *const c_void, chars: *mut u16) {
			let _ = unsafe { Box::from_raw_parts(chars.cast::<u8>(), data as usize * 2) };
		}

		extern "C" fn size_of_external_string(data: *const c_void, _: *const u16, _: MallocSizeOf) -> usize {
			data as usize
		}

		static EXTERNAL_STRING_CALLBACKS_TRAPS: JSExternalStringCallbacksTraps = JSExternalStringCallbacksTraps {
			finalize: Some(finalise_external_string),
			sizeOfBuffer: Some(size_of_external_string),
		};

		let vec = string.into_bytes();
		let boxed = vec.into_boxed_slice();

		let (chars, len) = unsafe { Box::into_raw_parts(boxed) };

		unsafe {
			let callbacks = CreateJSExternalStringCallbacks(&EXTERNAL_STRING_CALLBACKS_TRAPS, len as *mut c_void);
			let jsstr = JS_NewExternalString(cx.as_ptr(), chars.cast::<u16>(), len / 2, callbacks);

			if jsstr.is_null() {
				let slice = slice::from_raw_parts_mut(chars, len);
				let boxed = Box::from_raw(slice);
				let vec = Vec::from(boxed);
				Err(WString::from_utf16_unchecked(vec))
			} else {
				Ok(String::from(cx.root_string(jsstr)))
			}
		}
	}

	/// Returns a slice of a [String] as a new [String].
	pub fn slice<'cx>(&self, cx: &'cx Context, range: Range<usize>) -> String<'cx> {
		let Range { start, end } = range;
		String::from(cx.root_string(unsafe { JS_NewDependentString(cx.as_ptr(), self.handle().into(), start, end) }))
	}

	/// Concatenates two [String]s into a new [String].
	/// The resultant [String] is not linear.
	pub fn concat<'cx>(&self, cx: &'cx Context, other: &String) -> String<'cx> {
		String::from(
			cx.root_string(unsafe { JS_ConcatStrings(cx.as_ptr(), self.handle().into(), other.handle().into()) }),
		)
	}

	/// Compares two [String]s.
	pub fn compare(&self, cx: &Context, other: &String) -> i32 {
		let mut result = 0;
		unsafe { JS_CompareStrings(cx.as_ptr(), self.get(), other.get(), &mut result) };
		result
	}

	/// Checks if a string is linear (contiguous) in memory.
	pub fn is_linear(&self) -> bool {
		unsafe { JS_StringIsLinear(self.get()) }
	}

	/// Checks if a string consists of only Latin-1 characters.
	pub fn is_latin1(&self) -> bool {
		unsafe { JS_DeprecatedStringHasLatin1Chars(self.get()) }
	}

	/// Checks if a string consists of UTF-16 characters.
	pub fn is_utf16(&self) -> bool {
		!self.is_latin1()
	}

	/// Returns the UTF-16 codepoint at the given character.
	/// Returns [None] if the string is not linear.
	pub fn char_at(&self, cx: &Context, index: usize) -> u16 {
		unsafe {
			let mut char = 0;
			JS_GetStringCharAt(cx.as_ptr(), self.get(), index, &mut char);
			char
		}
	}

	/// Converts the [String] into a [prim@slice] of Latin-1 characters.
	/// Returns [None] if the string contains non-Latin-1 characters.
	pub fn as_latin1(&self, cx: &Context) -> Option<&'s [u8]> {
		self.is_latin1().then(|| unsafe {
			let mut length = 0;
			let chars = JS_GetLatin1StringCharsAndLength(cx.as_ptr(), ptr::null(), self.get(), &mut length);
			slice::from_raw_parts(chars, length)
		})
	}

	/// Converts the [String] into a [WStr].
	/// Returns [None] if the string contains only Latin-1 characters.
	pub fn as_wstr(&self, cx: &Context) -> Option<&'s WStr<NativeEndian>> {
		self.is_utf16()
			.then(|| unsafe {
				let mut length = 0;
				let chars = JS_GetTwoByteStringCharsAndLength(cx.as_ptr(), ptr::null(), self.get(), &mut length);
				let slice = slice::from_raw_parts(chars, length);
				cast_slice(slice)
			})
			.map(|bytes| WStr::from_utf16(bytes).unwrap())
	}

	pub fn as_ref(&self, cx: &Context) -> StringRef<'s> {
		let mut length = 0;
		if self.is_latin1() {
			let chars = unsafe { JS_GetLatin1StringCharsAndLength(cx.as_ptr(), ptr::null(), self.get(), &mut length) };
			StringRef::Latin1(unsafe { ByteStr::from_unchecked(slice::from_raw_parts(chars, length)) })
		} else {
			let mut length = 0;
			let chars = unsafe { JS_GetTwoByteStringCharsAndLength(cx.as_ptr(), ptr::null(), self.get(), &mut length) };
			let slice = unsafe { slice::from_raw_parts(chars, length) };
			StringRef::Utf16(WStr::from_utf16(cast_slice(slice)).unwrap())
		}
	}

	/// Converts a [String] to an owned [String](RustString).
	pub fn to_owned(&self, cx: &Context) -> RustString {
		if let Some(chars) = self.as_latin1(cx) {
			let mut string = RustString::with_capacity(chars.len());
			string.extend(chars.iter().map(|c| *c as char));
			string
		} else {
			let string = self.as_wstr(cx).unwrap();
			string.to_utf8()
		}
	}
}

impl<'s> From<Local<'s, *mut JSString>> for String<'s> {
	fn from(str: Local<'s, *mut JSString>) -> String<'s> {
		String { str }
	}
}

impl<'s> Deref for String<'s> {
	type Target = Local<'s, *mut JSString>;

	fn deref(&self) -> &Self::Target {
		&self.str
	}
}

impl<'s> DerefMut for String<'s> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.str
	}
}
