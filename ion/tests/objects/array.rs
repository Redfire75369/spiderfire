use std::ptr;

use mozjs::jsapi::{JS_NewGlobalObject, JSAutoRealm, OnNewGlobalHookOption};
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};

use ion::{Array, Context, Value};
use ion::conversions::FromValue;
use ion::flags::PropertyFlags;

#[test]
fn array() {
	let engine = JSEngine::init().unwrap();
	let runtime = Runtime::new(engine.handle());
	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(runtime.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _realm = JSAutoRealm::new(runtime.cx(), global);

	let cx = &Context::from_runtime(&runtime);

	let mut array = Array::new(cx);
	array.set(cx, 0, &Value::null(cx));
	array.define(cx, 2, &Value::undefined(cx), PropertyFlags::all());

	let value1 = array.get(cx, 0).unwrap();
	let value2 = array.get(cx, 2).unwrap();
	assert_eq!(String::from("null"), String::from_value(cx, &value1, false, ()).unwrap());
	assert_eq!(String::from("undefined"), String::from_value(cx, &value2, false, ()).unwrap());

	assert!(array.delete(cx, 0));
	assert!(array.delete(cx, 2));
	assert!(array.get(cx, 0).is_none());
	assert!(array.get(cx, 2).is_some());
}
