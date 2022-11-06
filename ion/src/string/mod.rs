/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

// TODO: Add Support for JSExternalString

use std::{ptr, slice};
use std::ops::{Deref, Range};
use std::string::String as RustString;

use bytemuck::cast_slice;
use byteorder::NativeEndian;
use mozjs::jsapi::{
	JS_CompareStrings, JS_ConcatStrings, JS_DeprecatedStringHasLatin1Chars, JS_GetEmptyString, JS_GetLatin1StringCharsAndLength, JS_GetStringCharAt,
	JS_GetTwoByteStringCharsAndLength, JS_NewDependentString, JS_NewUCStringCopyN, JS_StringIsLinear, JSString,
};
use utf16string::WStr;

use crate::{Context, Local};

#[derive(Debug)]
pub struct String<'s> {
	string: Local<'s, *mut JSString>,
}

impl<'s> String<'s> {
	pub fn empty<'cx>(cx: &'cx Context) -> String<'cx> {
		String::from(cx.root_string(unsafe { JS_GetEmptyString(**cx) }))
	}

	pub fn new<'cx>(cx: &'cx Context, string: &str) -> Option<String<'cx>> {
		let mut utf16: Vec<u16> = Vec::with_capacity(string.len());
		utf16.extend(string.encode_utf16());
		let jsstr = unsafe { JS_NewUCStringCopyN(**cx, utf16.as_ptr(), utf16.len()) };
		if !jsstr.is_null() {
			Some(String::from(cx.root_string(jsstr)))
		} else {
			None
		}
	}

	pub fn slice<'cx: 's>(&self, cx: &'cx Context, range: &Range<usize>) -> Option<String<'s>> {
		self.is_linear().then(|| {
			let Range { start, end } = range;
			String::from(cx.root_string(unsafe { JS_NewDependentString(**cx, self.handle().into(), *start, *end) }))
		})
	}

	pub fn concat<'cx: 's>(&self, cx: &'cx Context, other: &String) -> Option<String<'s>> {
		(self.is_linear() && other.is_linear())
			.then(|| String::from(cx.root_string(unsafe { JS_ConcatStrings(**cx, self.handle().into(), other.handle().into()) })))
	}

	pub fn compare(&self, cx: &Context, other: &String) -> Option<i32> {
		self.is_linear().then(|| {
			let mut result = 0;
			unsafe { JS_CompareStrings(**cx, ***self, ***other, &mut result) };
			result
		})
	}

	pub fn is_linear(&self) -> bool {
		unsafe { JS_StringIsLinear(***self) }
	}

	pub fn is_latin1(&self) -> bool {
		unsafe { JS_DeprecatedStringHasLatin1Chars(***self) }
	}

	pub fn is_utf16(&self) -> bool {
		!self.is_latin1()
	}

	pub fn char_at(&self, cx: &Context, index: usize) -> Option<u16> {
		self.is_linear().then(|| unsafe {
			let mut char = 0;
			JS_GetStringCharAt(**cx, ***self, index, &mut char);
			char
		})
	}

	pub fn as_latin1(&self, cx: &Context) -> Option<&'s [u8]> {
		(self.is_latin1() && self.is_linear()).then(|| unsafe {
			let mut length = 0;
			let chars = JS_GetLatin1StringCharsAndLength(**cx, ptr::null(), ***self, &mut length);
			slice::from_raw_parts(chars, length)
		})
	}

	pub fn as_wstr(&self, cx: &Context) -> Option<&'s WStr<NativeEndian>> {
		(self.is_utf16() && self.is_linear())
			.then(|| unsafe {
				let mut length = 0;
				let chars = JS_GetTwoByteStringCharsAndLength(**cx, ptr::null(), ***self, &mut length);
				let slice = slice::from_raw_parts(chars, length);
				cast_slice(slice)
			})
			.and_then(|bytes| WStr::from_utf16(bytes).ok())
	}

	pub fn to_owned_string(&self, cx: &Context) -> Option<RustString> {
		if let Some(chars) = self.as_latin1(cx) {
			let mut string = RustString::with_capacity(chars.len());
			string.extend(chars.iter().map(|c| *c as char));
			Some(string)
		} else {
			self.as_wstr(cx).map(WStr::to_utf8)
		}
	}
}

impl<'s> From<Local<'s, *mut JSString>> for String<'s> {
	fn from(string: Local<'s, *mut JSString>) -> String<'s> {
		String { string }
	}
}

impl<'s> Deref for String<'s> {
	type Target = Local<'s, *mut JSString>;

	fn deref(&self) -> &Self::Target {
		&self.string
	}
}
