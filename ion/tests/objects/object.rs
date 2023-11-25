use mozjs::jsapi::JSAutoRealm;
use mozjs::rust::{JSEngine, Runtime};

use ion::{Context, Object, OwnedKey, Value};
use ion::conversions::FromValue;
use ion::flags::PropertyFlags;
use ion::objects::default_new_global;

#[test]
fn object() {
	let engine = JSEngine::init().unwrap();
	let runtime = Runtime::new(engine.handle());

	let cx = &Context::from_runtime(&runtime);
	let global = default_new_global(cx);
	let _realm = JSAutoRealm::new(runtime.cx(), global.handle().get());

	let mut object = Object::new(cx);
	object.set(cx, "key1", &Value::null(cx));
	object.define(cx, "key2", &Value::undefined(cx), PropertyFlags::all());

	let value1 = object.get(cx, "key1").unwrap();
	let value2 = object.get(cx, "key2").unwrap();
	assert_eq!(
		String::from("null"),
		String::from_value(cx, &value1, false, ()).unwrap()
	);
	assert_eq!(
		String::from("undefined"),
		String::from_value(cx, &value2, false, ()).unwrap()
	);

	let keys = object.keys(cx, None);
	for (i, key) in keys.enumerate() {
		let expected = format!("key{}", i + 1);
		assert_eq!(key.to_owned_key(cx), OwnedKey::String(expected));
	}

	assert!(object.delete(cx, "key1"));
	assert!(object.delete(cx, "key2"));
	assert!(object.get(cx, "key1").is_none());
	assert!(object.get(cx, "key2").is_some());
}
