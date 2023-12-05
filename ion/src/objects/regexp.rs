/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use mozjs::glue::JS_GetRegExpFlags;
use mozjs::jsapi::{
	CheckRegExpSyntax, ExecuteRegExp, ExecuteRegExpNoStatics, GetRegExpSource, Heap, JSObject, NewUCRegExpObject,
	ObjectIsRegExp,
};
use mozjs::jsapi::RegExpFlags as REFlags;
use mozjs::jsval::{JSVal, UndefinedValue};
use mozjs::rust::MutableHandle;

use crate::{Context, Object, Root, Value};
use crate::flags::RegExpFlags;

#[derive(Debug)]
pub struct RegExp {
	re: Root<Box<Heap<*mut JSObject>>>,
}

impl RegExp {
	/// Creates a new [RegExp] object with the given source and flags.
	pub fn new(cx: &Context, source: &str, flags: RegExpFlags) -> Option<RegExp> {
		let source: Vec<u16> = source.encode_utf16().collect();
		let regexp = unsafe { NewUCRegExpObject(cx.as_ptr(), source.as_ptr(), source.len(), flags.into()) };
		NonNull::new(regexp).map(|re| RegExp { re: cx.root(re.as_ptr()) })
	}

	/// Creates a [RegExp] from an object.
	/// Returns [None] if it is not a [RegExp].
	pub fn from(cx: &Context, object: Root<Box<Heap<*mut JSObject>>>) -> Option<RegExp> {
		if RegExp::is_regexp(cx, &object) {
			Some(RegExp { re: object })
		} else {
			None
		}
	}

	/// Creates a [RegExp] from an object.
	///
	/// ### Safety
	/// Object must be a [RegExp].
	pub unsafe fn from_unchecked(object: Root<Box<Heap<*mut JSObject>>>) -> RegExp {
		RegExp { re: object }
	}

	pub fn source(&self, cx: &Context) -> crate::String {
		crate::String::from(cx.root(unsafe { GetRegExpSource(cx.as_ptr(), self.handle().into()) }))
	}

	pub fn flags(&self, cx: &Context) -> RegExpFlags {
		let mut flags = REFlags { flags_: 0 };
		unsafe {
			JS_GetRegExpFlags(cx.as_ptr(), self.handle().into(), &mut flags);
		}
		flags.into()
	}

	pub fn to_string(&self, cx: &Context) -> String {
		format!("/{}/{}", self.source(cx).to_owned(cx), self.flags(cx))
	}

	pub fn execute_test(&self, cx: &Context, string: &str, index: &mut usize) -> bool {
		rooted!(in(cx.as_ptr()) let mut rval = UndefinedValue());
		self.execute(cx, string, index, true, rval.handle_mut(), true) && rval.to_boolean()
	}

	pub fn execute_test_no_static(&self, cx: &Context, string: &str, index: &mut usize) -> bool {
		rooted!(in(cx.as_ptr()) let mut rval = UndefinedValue());
		self.execute(cx, string, index, true, rval.handle_mut(), false) && rval.to_boolean()
	}

	pub fn execute_match(&self, cx: &Context, string: &str, index: &mut usize) -> Option<Value> {
		rooted!(in(cx.as_ptr()) let mut rval = UndefinedValue());
		if self.execute(cx, string, index, false, rval.handle_mut(), true) {
			Some(Value::from(cx.root(rval.get())))
		} else {
			None
		}
	}

	pub fn execute_match_no_static(&self, cx: &Context, string: &str, index: &mut usize) -> Option<Value> {
		rooted!(in(cx.as_ptr()) let mut rval = UndefinedValue());
		if self.execute(cx, string, index, false, rval.handle_mut(), false) {
			Some(Value::from(cx.root(rval.get())))
		} else {
			None
		}
	}

	fn execute(
		&self, cx: &Context, string: &str, index: &mut usize, test: bool, rval: MutableHandle<JSVal>, with_static: bool,
	) -> bool {
		let string: Vec<u16> = string.encode_utf16().collect();
		if with_static {
			let global = Object::global(cx);
			unsafe {
				ExecuteRegExp(
					cx.as_ptr(),
					global.handle().into(),
					self.handle().into(),
					string.as_ptr(),
					string.len(),
					index,
					test,
					rval.into(),
				)
			}
		} else {
			unsafe {
				ExecuteRegExpNoStatics(
					cx.as_ptr(),
					self.handle().into(),
					string.as_ptr(),
					string.len(),
					index,
					test,
					rval.into(),
				)
			}
		}
	}

	pub fn check_syntax(cx: &Context, source: &str, flags: RegExpFlags) -> Result<(), Value> {
		let source: Vec<u16> = source.encode_utf16().collect();
		rooted!(in(cx.as_ptr()) let mut error = UndefinedValue());
		let check = unsafe {
			CheckRegExpSyntax(
				cx.as_ptr(),
				source.as_ptr(),
				source.len(),
				flags.into(),
				error.handle_mut().into(),
			)
		};
		if check && error.is_undefined() {
			Ok(())
		} else {
			Err(Value::from(cx.root(error.get())))
		}
	}

	/// Checks if a [raw object](*mut JSObject) is a regexp.
	pub fn is_regexp_raw(cx: &Context, object: *mut JSObject) -> bool {
		rooted!(in(cx.as_ptr()) let object = object);
		let mut is_regexp = false;
		(unsafe { ObjectIsRegExp(cx.as_ptr(), object.handle().into(), &mut is_regexp) }) && is_regexp
	}

	/// Checks if an object is a regexp.
	pub fn is_regexp(cx: &Context, object: &Root<Box<Heap<*mut JSObject>>>) -> bool {
		let mut is_regexp = false;
		(unsafe { ObjectIsRegExp(cx.as_ptr(), object.handle().into(), &mut is_regexp) }) && is_regexp
	}
}

impl Deref for RegExp {
	type Target = Root<Box<Heap<*mut JSObject>>>;

	fn deref(&self) -> &Self::Target {
		&self.re
	}
}

impl DerefMut for RegExp {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.re
	}
}
