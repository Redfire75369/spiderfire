/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::CString;
use std::ptr;

use mozjs::jsapi::{JS_DefineFunctions, JS_NewPlainObject, JS_ReportErrorUTF8, JSFunctionSpec, JSNativeWrapper, Value};
use mozjs::jsval::ObjectValue;

use ion::functions::arguments::Arguments;
use ion::functions::macros::{IonContext, IonResult};
use ion::functions::specs::{create_function_spec, NULL_SPEC};
use ion::objects::object::IonObject;
use runtime::config::{Config, LogLevel};
use runtime::modules::{compile_module, register_module};

const ASSERT_SOURCE: &str = include_str!("assert.js");

#[js_fn]
unsafe fn assert(assertion: Option<bool>, message: Option<String>) -> IonResult<()> {
	match assertion {
		Some(b) => match message {
			Some(m) => assert!(b, "Assertion Failed: {}", m),
			None => assert!(b, "Assertion Failed"),
		},
		None => (),
	};
	Ok(())
}

#[js_fn]
unsafe fn debug_assert(assertion: Option<bool>, message: Option<String>) -> IonResult<()> {
	if Config::global().log_level == LogLevel::Debug {
		match assertion {
			Some(b) => match message {
				Some(m) => assert!(b, "Assertion Failed: {}", m),
				None => assert!(b, "Assertion Failed"),
			},
			None => (),
		};
	}
	Ok(())
}

const METHODS: &[JSFunctionSpec] = &[
	create_function_spec(
		"assert\0",
		JSNativeWrapper {
			op: Some(assert),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"debugAssert\0",
		JSNativeWrapper {
			op: Some(debug_assert),
			info: ptr::null_mut(),
		},
		1,
	),
	NULL_SPEC,
];

pub fn init_assert(cx: IonContext, mut global: IonObject) -> bool {
	let internal_key = String::from("______assertInternal______");
	unsafe {
		rooted!(in(cx) let assert_module = JS_NewPlainObject(cx));
		if JS_DefineFunctions(cx, assert_module.handle().into(), METHODS.as_ptr()) {
			if global.define(cx, internal_key, ObjectValue(assert_module.get()), 0) {
				return register_module(
					cx,
					&String::from("assert"),
					compile_module(cx, &String::from("assert"), None, &String::from(ASSERT_SOURCE)).unwrap(),
				);
				// && global.delete(cx, internal_key);
			}
		}
		false
	}
}
