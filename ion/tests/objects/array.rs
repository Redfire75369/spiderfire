use mozjs::jsapi::JSAutoRealm;
use mozjs::rust::{JSEngine, Runtime};

use ion::{Array, Context, Value};
use ion::conversions::FromValue;
use ion::flags::PropertyFlags;
use ion::objects::default_new_global;

#[test]
fn array() {
	let engine = JSEngine::init().unwrap();
	let runtime = Runtime::new(engine.handle());

	let cx = &Context::from_runtime(&runtime);
	let global = default_new_global(cx);
	let _realm = JSAutoRealm::new(runtime.cx(), global.handle().get());

	let mut array = Array::new(cx);
	array.set(cx, 0, &Value::null(cx));
	array.define(cx, 2, &Value::undefined(cx), PropertyFlags::all());

	let value1 = array.get(cx, 0).unwrap();
	let value2 = array.get(cx, 2).unwrap();
	assert_eq!(
		String::from("null"),
		String::from_value(cx, &value1, false, ()).unwrap()
	);
	assert_eq!(
		String::from("undefined"),
		String::from_value(cx, &value2, false, ()).unwrap()
	);

	assert!(array.delete(cx, 0));
	assert!(array.delete(cx, 2));
	assert!(array.get(cx, 0).is_none());
	assert!(array.get(cx, 2).is_some());
}
