/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::{JSFunctionSpec, SameValue};

use ion::{Context, Error, Function, Object, Result, Value};
use runtime::modules::NativeModule;

fn assert_internal(message: Option<String>) -> Result<()> {
	let error = match message {
		Some(msg) => format!("Assertion Failed: {}", msg),
		None => String::from("Assertion Failed"),
	};
	Err(Error::new(&error, None))
}

#[js_fn]
fn ok(assertion: Option<bool>, message: Option<String>) -> Result<()> {
	if let Some(true) = assertion {
		Ok(())
	} else {
		assert_internal(message)
	}
}

#[js_fn]
unsafe fn equals(cx: &Context, actual: Value, expected: Value, message: Option<String>) -> Result<()> {
	let mut same = false;
	if SameValue(**cx, actual.handle().into(), expected.handle().into(), &mut same) {
		if same {
			Ok(())
		} else {
			assert_internal(message)
		}
	} else {
		Err(Error::new("", None))
	}
}

#[js_fn]
unsafe fn throws(cx: &Context, func: Function, message: Option<String>) -> Result<()> {
	if func.call(cx, &Object::global(cx), &[]).is_err() {
		assert_internal(message)
	} else {
		Ok(())
	}
}

#[js_fn]
fn fail(message: Option<String>) -> Result<()> {
	assert_internal(message)
}

const FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(ok, 0),
	function_spec!(equals, 2),
	function_spec!(throws, 1),
	function_spec!(fail, 0),
	JSFunctionSpec::ZERO,
];

#[derive(Default)]
pub struct Assert;

impl NativeModule for Assert {
	const NAME: &'static str = "assert";
	const SOURCE: &'static str = include_str!("assert.js");

	fn module<'cx>(cx: &'cx Context) -> Option<Object<'cx>> {
		let mut assert = Object::new(cx);
		if assert.define_methods(cx, FUNCTIONS) {
			return Some(assert);
		}
		None
	}
}
