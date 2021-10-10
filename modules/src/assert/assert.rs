/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::{IonContext, IonResult};
use ion::error::IonError;
use ion::functions::arguments::Arguments;
use ion::functions::function::IonFunction;
use ion::objects::object::IonObject;
use mozjs::jsapi::{CurrentGlobalOrNull, JS_DefineFunctions, JS_NewPlainObject, JSFunctionSpec, SameValue, Value};
use mozjs::jsval::ObjectValue;
use runtime::modules::IonModule;

const ASSERT_SOURCE: &str = include_str!("assert.js");

fn assert_internal(message: Option<String>) -> IonResult<()> {
	Err(IonError::Error(match message {
		Some(msg) => format!("Assertion Failed: {}", msg),
		None => String::from("Assertion Failed"),
	}))
}

#[js_fn]
unsafe fn ok(assertion: Option<bool>, message: Option<String>) -> IonResult<()> {
	if let Some(true) = assertion {
		assert_internal(message)
	} else {
		Ok(())
	}
}

#[js_fn]
unsafe fn equals(cx: IonContext, actual: Value, expected: Value, message: Option<String>) -> IonResult<()> {
	let mut same = false;
	rooted!(in(cx) let actual = actual);
	rooted!(in(cx) let expected = expected);
	if SameValue(cx, actual.handle().into(), expected.handle().into(), &mut same) {
		if !same {
			assert_internal(message)
		} else {
			Ok(())
		}
	} else {
		Err(IonError::None)
	}
}

#[js_fn]
unsafe fn throws(cx: IonContext, func: IonFunction, message: Option<String>) -> IonResult<()> {
	if func.call_with_vec(cx, IonObject::from(CurrentGlobalOrNull(cx)), vec![]).is_err() {
		assert_internal(message)
	} else {
		Ok(())
	}
}

#[js_fn]
unsafe fn fail(message: Option<String>) -> IonResult<()> {
	assert_internal(message)
}

const METHODS: &[JSFunctionSpec] = &[
	function_spec!(ok, 0),
	function_spec!(equals, 2),
	function_spec!(throws, 1),
	function_spec!(fail, 0),
	JSFunctionSpec::ZERO,
];

/*
 * TODO: Remove JS Wrapper, Stop Global Scope Pollution, Use CreateEmptyModule and AddModuleExport
 * TODO: Waiting on https://bugzilla.mozilla.org/show_bug.cgi?id=1722802
 */
pub unsafe fn init(cx: IonContext, mut global: IonObject) -> bool {
	let internal_key = String::from("______assertInternal______");
	rooted!(in(cx) let assert_module = JS_NewPlainObject(cx));
	if JS_DefineFunctions(cx, assert_module.handle().into(), METHODS.as_ptr()) {
		if global.define(cx, internal_key, ObjectValue(assert_module.get()), 0) {
			let module = IonModule::compile(cx, "assert", None, ASSERT_SOURCE).unwrap();
			module.register("assert")
		} else {
			false
		}
	} else {
		false
	}
}
