use std::f64::consts::PI;

use chrono::{TimeZone, Utc};
use mozjs::jsapi::JSAutoRealm;
use mozjs::jsval::Int32Value;
use mozjs::rust::{JSEngine, Runtime};

use ion::{Array, Context, Date, Object, Promise, Value};
use ion::conversions::{FromValue, ToValue};
use ion::conversions::ConversionBehavior;
use ion::object::default_new_global;

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
	let result = bool::from_value(cx, &value, true, ());
	assert!(!result.unwrap());

	let value = Value::i32(cx, 0);
	let result = bool::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = bool::from_value(cx, &value, false, ());
	assert!(!result.unwrap());

	let value = Value::f64(cx, PI);
	let result = bool::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = bool::from_value(cx, &value, false, ());
	assert!(result.unwrap());

	let value = Value::string(cx, "");
	let result = bool::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = bool::from_value(cx, &value, false, ());
	assert!(!result.unwrap());

	let value = Value::string(cx, "spider");
	let result = bool::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = bool::from_value(cx, &value, false, ());
	assert!(result.unwrap());

	let value = Value::undefined(cx);
	let result = bool::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = bool::from_value(cx, &value, false, ());
	assert!(!result.unwrap());

	let value = Value::null(cx);
	let result = bool::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = bool::from_value(cx, &value, false, ());
	assert!(!result.unwrap());

	let object = Object::new(cx);
	let value = object.as_value(cx);
	let result = bool::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = bool::from_value(cx, &value, false, ());
	assert!(result.unwrap());

	let array = Array::new(cx);
	let value = array.as_value(cx);
	let result = bool::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = bool::from_value(cx, &value, false, ());
	assert!(result.unwrap());
}

fn test_integers(cx: &Context) {
	let value = Value::bool(cx, true);
	let result = i32::from_value(cx, &value, true, ConversionBehavior::EnforceRange);
	assert!(result.is_err());
	let result = i32::from_value(cx, &value, false, ConversionBehavior::EnforceRange);
	assert_eq!(result.unwrap(), 1);

	let value = Value::i32(cx, 255);
	let result = u8::from_value(cx, &value, true, ConversionBehavior::EnforceRange);
	assert_eq!(result.unwrap(), 255);

	let value = Value::string(cx, "spider");
	let result = u16::from_value(cx, &value, true, ConversionBehavior::EnforceRange);
	assert!(result.is_err());
	let result = u16::from_value(cx, &value, false, ConversionBehavior::EnforceRange);
	assert!(result.is_err());

	let value = Value::string(cx, "-64");
	let result = i64::from_value(cx, &value, true, ConversionBehavior::EnforceRange);
	assert!(result.is_err());
	let result = i64::from_value(cx, &value, false, ConversionBehavior::EnforceRange);
	assert_eq!(result.unwrap(), -64);

	let value = Value::undefined(cx);
	let result = i64::from_value(cx, &value, true, ConversionBehavior::EnforceRange);
	assert!(result.is_err());
	let result = i64::from_value(cx, &value, false, ConversionBehavior::EnforceRange);
	assert!(result.is_err());

	let value = Value::null(cx);
	let result = i64::from_value(cx, &value, true, ConversionBehavior::EnforceRange);
	assert!(result.is_err());
	let result = i64::from_value(cx, &value, false, ConversionBehavior::EnforceRange);
	assert_eq!(result.unwrap(), 0);

	let object = Object::new(cx);
	let value = object.as_value(cx);
	let result = u32::from_value(cx, &value, true, ConversionBehavior::EnforceRange);
	assert!(result.is_err());
	let result = u32::from_value(cx, &value, false, ConversionBehavior::EnforceRange);
	assert!(result.is_err());
}

fn test_strings(cx: &Context) {
	let value = Value::bool(cx, false);
	let result = String::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = String::from_value(cx, &value, false, ());
	assert_eq!(&result.unwrap(), "false");

	let value = Value::f64(cx, 1.5);
	let result = String::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = String::from_value(cx, &value, false, ());
	assert_eq!(&result.unwrap(), "1.5");

	let value = Value::string(cx, "spider");
	let result = String::from_value(cx, &value, true, ());
	assert_eq!(&result.unwrap(), "spider");

	let value = Value::undefined(cx);
	let result = String::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = String::from_value(cx, &value, false, ());
	assert_eq!(&result.unwrap(), "undefined");

	let value = Value::null(cx);
	let result = String::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = String::from_value(cx, &value, false, ());
	assert_eq!(&result.unwrap(), "null");

	let object = Object::new(cx);
	let value = object.as_value(cx);
	let result = String::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = String::from_value(cx, &value, false, ());
	assert_eq!(&result.unwrap(), "[object Object]");
}

fn test_objects(cx: &Context) {
	let value = Value::bool(cx, false);
	let result = Object::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = Object::from_value(cx, &value, false, ());
	assert!(result.is_err());

	let value = Value::f64(cx, 144.0);
	let result = Object::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = Object::from_value(cx, &value, false, ());
	assert!(result.is_err());

	let value = Value::string(cx, "spider");
	let result = Object::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = Object::from_value(cx, &value, false, ());
	assert!(result.is_err());

	let value = Value::undefined(cx);
	let result = Object::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = Object::from_value(cx, &value, false, ());
	assert!(result.is_err());

	let value = Value::null(cx);
	let result = Object::from_value(cx, &value, true, ());
	assert!(result.is_err());
	let result = Object::from_value(cx, &value, false, ());
	assert!(result.is_err());

	let object = Object::new(cx);
	let value = object.as_value(cx);
	let result = Object::from_value(cx, &value, true, ());
	assert!(result.is_ok());

	let array = Array::new(cx);
	let value = array.as_value(cx);
	let result = Array::from_value(cx, &value, true, ());
	assert!(result.is_ok());

	let timestamp = Utc.timestamp_millis_opt(Utc::now().timestamp_millis()).unwrap();
	let date = Date::from_date(cx, timestamp);
	let value = date.as_value(cx);
	let result = Date::from_value(cx, &value, true, ());
	assert_eq!(result.unwrap().to_date(cx).unwrap(), timestamp);

	let promise = Promise::new(cx);
	let value = promise.as_value(cx);
	let result = Promise::from_value(cx, &value, true, ());
	assert!(result.is_ok());
}

fn test_options(cx: &Context) {
	type Opt = Option<bool>;

	let value = Value::bool(cx, true);
	let result = Opt::from_value(cx, &value, true, ());
	assert_eq!(result.unwrap(), Some(true));

	let value = Value::undefined(cx);
	let result = Opt::from_value(cx, &value, true, ());
	assert_eq!(result.unwrap(), None);

	let value = Value::null(cx);
	let result = Opt::from_value(cx, &value, true, ());
	assert_eq!(result.unwrap(), None);
}

fn test_vec(cx: &Context) {
	let int_vec = vec![1, 256, -65536, 2147483647];
	let vec: Vec<_> = int_vec.iter().map(|i| Int32Value(*i)).collect();
	let array = Array::from_slice(cx, vec.as_slice());
	let value = array.as_value(cx);

	let result = <Vec<i32>>::from_value(cx, &value, true, ConversionBehavior::EnforceRange);
	assert_eq!(result.unwrap(), int_vec);
}
