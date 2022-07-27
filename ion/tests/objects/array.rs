use std::ptr;

use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{JS_NewGlobalObject, JSAutoRealm, OnNewGlobalHookOption};
use mozjs::rooted;
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};
use mozjs::rust::jsapi_wrapped::JS_ValueToSource;

use ion::{Array, Value};
use ion::flags::PropertyFlags;

#[test]
fn array() {
	let engine = JSEngine::init().unwrap();
	let runtime = Runtime::new(engine.handle());
	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(runtime.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _realm = JSAutoRealm::new(runtime.cx(), global);

	let cx = runtime.cx();

	let mut array = Array::new(cx);
	array.set(cx, 0, *Value::null());
	array.define(cx, 2, *Value::undefined(), PropertyFlags::all());
	rooted!(in(cx) let value1 = array.get(cx, 0).unwrap());
	rooted!(in(cx) let value2 = array.get(cx, 2).unwrap());
	unsafe {
		assert_eq!(String::from("null"), jsstr_to_string(cx, JS_ValueToSource(cx, value1.handle())));
		assert_eq!(String::from("(void 0)"), jsstr_to_string(cx, JS_ValueToSource(cx, value2.handle())));
	}

	assert!(array.delete(cx, 0));
	assert!(array.delete(cx, 2));
	assert!(array.get(cx, 0).is_none());
	assert!(array.get(cx, 2).is_some());
}
