use ion::conversions::FromValue;
use ion::flags::PropertyFlags;
use ion::{Object, OwnedKey, Value};
use ion::utils::test::TestRuntime;

#[test]
fn object() {
	let rt = TestRuntime::new();
	let cx = &rt.cx;

	let object = Object::new(cx);
	object.set(cx, "key1", &Value::null(cx));
	object.define(cx, "key2", &Value::undefined_handle(), PropertyFlags::all());

	let value1 = object.get(cx, "key1").unwrap().unwrap();
	let value2 = object.get(cx, "key2").unwrap().unwrap();
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
		assert_eq!(key.to_owned_key(cx).unwrap(), OwnedKey::String(expected));
	}

	assert!(object.delete(cx, "key1"));
	assert!(object.delete(cx, "key2"));
	assert!(object.get(cx, "key1").unwrap().is_none());
	assert!(object.get(cx, "key2").unwrap().is_some());
}
