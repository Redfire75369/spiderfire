/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use mozjs::jsapi::JSFunctionSpec;
use mozjs::rust::JSEngine;
use mozjs::rust::Runtime as RustRuntime;

use ion::{Context, Error, Exception, Function, Object, Value};
use ion::module::Module;
use modules::Assert;
use runtime::{Runtime, RuntimeBuilder};
use runtime::config::{Config, CONFIG, LogLevel};
use runtime::modules::Loader;

const OK: (&str, &str) = ("ok", include_str!("scripts/assert/ok.js"));
const EQUALS: (&str, &str) = ("equals", include_str!("scripts/assert/equals.js"));
const THROWS: (&str, &str) = ("throws", include_str!("scripts/assert/throws.js"));
const FAIL: (&str, &str) = ("fail", include_str!("scripts/assert/fail.js"));

const EXCEPTION_STRING: &str = "_spidermonkey_exception_";

#[tokio::test]
async fn assert() {
	CONFIG.set(Config::default().log_level(LogLevel::Debug)).unwrap();

	let engine = JSEngine::init().unwrap();
	let rt = RustRuntime::new(engine.handle());

	let cx = &mut Context::from_runtime(&rt);
	let rt = RuntimeBuilder::new()
		.modules(Loader::default())
		.standard_modules(Assert)
		.microtask_queue()
		.build(cx);

	eval_module(&rt, rt.cx(), OK).await;
	eval_module(&rt, rt.cx(), EQUALS).await;
	eval_module(&rt, rt.cx(), THROWS).await;
	eval_module(&rt, rt.cx(), FAIL).await;
}

pub async fn eval_module(rt: &Runtime<'_>, cx: &Context, test: (&str, &str)) {
	let (test, script) = test;
	let filename = format!("{}.js", test);
	let path = format!("./tests/scripts/assert/{}.js", test);
	let error = format!("Assertion Failed: assert.{}", test);

	let result = Module::compile(cx, &filename, Some(Path::new(&path)), script);
	assert!(result.is_ok(), "Exception was thrown in: {}", filename);

	let (_, promise) = result.unwrap();
	assert!(promise.is_some());

	let on_rejected = Function::from_spec(cx, &ON_REJECTED);
	promise.unwrap().add_reactions(cx, None, Some(on_rejected));

	assert!(rt.run_event_loop().await.is_ok());

	let exception = rt.global().get(rt.cx(), EXCEPTION_STRING).unwrap();
	let exception = Exception::from_value(cx, &exception);
	match exception {
		Exception::Error(Error { ref message, .. }) => {
			assert_eq!(message, &error, "{}: {:#?}", filename, exception);
		}
		_ => {
			panic!("Exception was not an Error")
		}
	}
}

#[ion::js_fn]
fn on_rejected(cx: &Context, value: Value) {
	let mut global = Object::global(cx);
	global.set(cx, EXCEPTION_STRING, &value);
	Exception::clear(cx);
}

static ON_REJECTED: JSFunctionSpec = ion::function_spec!(on_rejected, "onRejected", 0);
