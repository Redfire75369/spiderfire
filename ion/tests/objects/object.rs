use std::ptr;

use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{JS_NewGlobalObject, JSAutoRealm, OnNewGlobalHookOption};
use mozjs::rooted;
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};
use mozjs::rust::jsapi_wrapped::JS_ValueToSource;

use ion::{Object, Value};
use ion::flags::PropertyFlags;

#[test]
fn object() {
	let engine = JSEngine::init().unwrap();
	let runtime = Runtime::new(engine.handle());
	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(runtime.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _realm = JSAutoRealm::new(runtime.cx(), global);

	let cx = runtime.cx();

	let mut object = Object::new(cx);
	object.set(cx, "key1", *Value::null());
	object.define(cx, "key2", *Value::undefined(), PropertyFlags::all());
	rooted!(in(cx) let value1 = object.get(cx, "key1").unwrap());
	rooted!(in(cx) let value2 = object.get(cx, "key2").unwrap());
	unsafe {
		assert_eq!(String::from("null"), jsstr_to_string(cx, JS_ValueToSource(cx, value1.handle())));
		assert_eq!(String::from("(void 0)"), jsstr_to_string(cx, JS_ValueToSource(cx, value2.handle())));
	}

	let keys = object.keys(cx, None);
	println!("{:?}", keys);

	assert!(object.delete(cx, "key1"));
	assert!(object.delete(cx, "key2"));
	assert!(object.get(cx, "key1").is_none());
	assert!(object.get(cx, "key2").is_some());
}
