use ion::conversions::FromValue;
use ion::flags::PropertyFlags;
use ion::{Array, Value};
use ion::utils::test::TestRuntime;

#[test]
fn array() {
	let rt = TestRuntime::new();
	let cx = &rt.cx;

	let array = Array::new(cx);
	array.set(cx, 0, &Value::null(cx));
	array.define(cx, 2, &Value::undefined_handle(), PropertyFlags::all());

	let value1 = array.get(cx, 0).unwrap().unwrap();
	let value2 = array.get(cx, 2).unwrap().unwrap();
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
	assert!(array.get(cx, 0).unwrap().is_none());
	assert!(array.get(cx, 2).unwrap().is_some());
}
