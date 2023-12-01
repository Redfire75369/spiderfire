use std::f64::consts::PI;

use chrono::{TimeZone, Utc};
use mozjs::jsapi::JSAutoRealm;
use mozjs::jsval::Int32Value;
use mozjs::rust::{JSEngine, Runtime};

use ion::{Array, Context, Date, Object, Promise, Value};
use ion::conversions::{FromValue, ToValue};
use ion::conversions::ConversionBehavior;
use ion::objects::default_new_global;

#[test]
fn from_value() {
	let engine = JSEngine::init().unwrap();
	let runtime = Runtime::new(engine.handle());

	let cx = &Context::from_runtime(&runtime);
	let global = default_new_global(cx);
	let _realm = JSAutoRealm::new(runtime.cx(), global.handle().get());

	test_booleans(cx);
	test_integers(cx);
	test_strings(cx);
	test_objects(cx);
	test_options(cx);
	test_vec(cx);
}

fn test_booleans(cx: &Context) {
	let value = Value::bool(cx, false);
	let result = bool::from_value(cx, &value, true, ()).unwrap();
	assert!(!result);

	let value = Value::i32(cx, 0);
	bool::from_value(cx, &value, true, ()).unwrap_err();
	let result = bool::from_value(cx, &value, false, ()).unwrap();
	assert!(!result);

	let value = Value::f64(cx, PI);
	bool::from_value(cx, &value, true, ()).unwrap_err();
	let result = bool::from_value(cx, &value, false, ()).unwrap();
	assert!(result);

	let value = Value::string(cx, "");
	bool::from_value(cx, &value, true, ()).unwrap_err();
	let result = bool::from_value(cx, &value, false, ()).unwrap();
	assert!(!result);

	let value = Value::string(cx, "spider");
	bool::from_value(cx, &value, true, ()).unwrap_err();
	let result = bool::from_value(cx, &value, false, ()).unwrap();
	assert!(result);

	let value = Value::undefined(cx);
	bool::from_value(cx, &value, true, ()).unwrap_err();
	let result = bool::from_value(cx, &value, false, ()).unwrap();
	assert!(!result);

	let value = Value::null(cx);
	bool::from_value(cx, &value, true, ()).unwrap_err();
	let result = bool::from_value(cx, &value, false, ()).unwrap();
	assert!(!result);

	let object = Object::new(cx);
	let value = object.to_value(cx).unwrap();
	bool::from_value(cx, &value, true, ()).unwrap_err();
	let result = bool::from_value(cx, &value, false, ()).unwrap();
	assert!(result);

	let value = Array::new(cx).to_value(cx).unwrap();
	bool::from_value(cx, &value, true, ()).unwrap_err();
	let result = bool::from_value(cx, &value, false, ()).unwrap();
	assert!(result);
}

fn test_integers(cx: &Context) {
	let value = Value::bool(cx, true);
	i32::from_value(cx, &value, true, ConversionBehavior::EnforceRange).unwrap_err();
	let result = i32::from_value(cx, &value, false, ConversionBehavior::EnforceRange).unwrap();
	assert_eq!(result, 1);

	let value = Value::i32(cx, 255);
	let result = u8::from_value(cx, &value, true, ConversionBehavior::EnforceRange).unwrap();
	assert_eq!(result, 255);

	let value = Value::string(cx, "spider");
	u16::from_value(cx, &value, true, ConversionBehavior::EnforceRange).unwrap_err();
	u16::from_value(cx, &value, false, ConversionBehavior::EnforceRange).unwrap_err();

	let value = Value::string(cx, "-64");
	i64::from_value(cx, &value, true, ConversionBehavior::EnforceRange).unwrap_err();
	let result = i64::from_value(cx, &value, false, ConversionBehavior::EnforceRange).unwrap();
	assert_eq!(result, -64);

	let value = Value::undefined(cx);
	i64::from_value(cx, &value, true, ConversionBehavior::EnforceRange).unwrap_err();
	i64::from_value(cx, &value, false, ConversionBehavior::EnforceRange).unwrap_err();

	let value = Value::null(cx);
	i64::from_value(cx, &value, true, ConversionBehavior::EnforceRange).unwrap_err();
	let result = i64::from_value(cx, &value, false, ConversionBehavior::EnforceRange).unwrap();
	assert_eq!(result, 0);

	let value = Object::new(cx).to_value(cx).unwrap();
	u32::from_value(cx, &value, true, ConversionBehavior::EnforceRange).unwrap_err();
	u32::from_value(cx, &value, false, ConversionBehavior::EnforceRange).unwrap_err();
}

fn test_strings(cx: &Context) {
	let value = Value::bool(cx, false);
	String::from_value(cx, &value, true, ()).unwrap_err();
	let result = String::from_value(cx, &value, false, ()).unwrap();
	assert_eq!(&result, "false");

	let value = Value::f64(cx, 1.5);
	String::from_value(cx, &value, true, ()).unwrap_err();
	let result = String::from_value(cx, &value, false, ());
	assert_eq!(&result.unwrap(), "1.5");

	let value = Value::string(cx, "spider");
	let result = String::from_value(cx, &value, true, ()).unwrap();
	assert_eq!(&result, "spider");

	let value = Value::undefined(cx);
	String::from_value(cx, &value, true, ()).unwrap_err();
	let result = String::from_value(cx, &value, false, ()).unwrap();
	assert_eq!(&result, "undefined");

	let value = Value::null(cx);
	String::from_value(cx, &value, true, ()).unwrap_err();
	let result = String::from_value(cx, &value, false, ()).unwrap();
	assert_eq!(&result, "null");

	let value = Object::new(cx).to_value(cx).unwrap();
	String::from_value(cx, &value, true, ()).unwrap_err();
	let result = String::from_value(cx, &value, false, ()).unwrap();
	assert_eq!(&result, "[object Object]");
}

fn test_objects(cx: &Context) {
	let value = Value::bool(cx, false);
	Object::from_value(cx, &value, true, ()).unwrap_err();
	Object::from_value(cx, &value, false, ()).unwrap_err();

	let value = Value::f64(cx, 144.0);
	Object::from_value(cx, &value, true, ()).unwrap_err();
	Object::from_value(cx, &value, false, ()).unwrap_err();

	let value = Value::string(cx, "spider");
	Object::from_value(cx, &value, true, ()).unwrap_err();
	Object::from_value(cx, &value, false, ()).unwrap_err();

	let value = Value::undefined(cx);
	Object::from_value(cx, &value, true, ()).unwrap_err();
	Object::from_value(cx, &value, false, ()).unwrap_err();

	let value = Value::null(cx);
	Object::from_value(cx, &value, true, ()).unwrap_err();
	Object::from_value(cx, &value, false, ()).unwrap_err();

	let object = Object::new(cx);
	let value = object.to_value(cx).unwrap();
	Object::from_value(cx, &value, true, ()).unwrap();

	let value = Array::new(cx).to_value(cx).unwrap();
	Array::from_value(cx, &value, true, ()).unwrap();

	let timestamp = Utc.timestamp_millis_opt(Utc::now().timestamp_millis()).unwrap();
	let value = Date::from_date(cx, timestamp).to_value(cx).unwrap();
	let result = Date::from_value(cx, &value, true, ()).unwrap();
	assert_eq!(result.to_date(cx).unwrap(), timestamp);

	let promise = Promise::new(cx);
	let value = promise.to_value(cx).unwrap();
	Promise::from_value(cx, &value, true, ()).unwrap();
}

fn test_options(cx: &Context) {
	type Opt = Option<bool>;

	let value = Value::bool(cx, true);
	let result = Opt::from_value(cx, &value, true, ()).unwrap();
	assert_eq!(result, Some(true));

	let value = Value::undefined(cx);
	let result = Opt::from_value(cx, &value, true, ()).unwrap();
	assert_eq!(result, None);

	let value = Value::null(cx);
	let result = Opt::from_value(cx, &value, true, ()).unwrap();
	assert_eq!(result, None);
}

fn test_vec(cx: &Context) {
	let int_vec = vec![1, 256, -65536, 2147483647];
	let vec: Vec<_> = int_vec.iter().map(|i| Int32Value(*i)).collect();
	let value = Array::from_slice(cx, vec.as_slice()).to_value(cx).unwrap();

	let result = <Vec<i32>>::from_value(cx, &value, true, ConversionBehavior::EnforceRange).unwrap();
	assert_eq!(result, int_vec);
}
