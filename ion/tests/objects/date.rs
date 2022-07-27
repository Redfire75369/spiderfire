use std::ptr;

use chrono::{TimeZone, Utc};
use mozjs::jsapi::{JS_NewGlobalObject, JSAutoRealm, OnNewGlobalHookOption};
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};

use ion::Date;

const EPOCH: i64 = 0; // 01 January 1970
const POST_EPOCH: i64 = 1615766400; // 15 March 2021
const PRE_EPOCH: i64 = -1615766400; // 20 October 1918

#[test]
fn date() {
	let engine = JSEngine::init().unwrap();
	let runtime = Runtime::new(engine.handle());
	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(runtime.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _realm = JSAutoRealm::new(runtime.cx(), global);

	let cx = runtime.cx();

	let epoch = Date::from_date(cx, Utc.timestamp_millis(EPOCH));
	let post_epoch = Date::from_date(cx, Utc.timestamp_millis(POST_EPOCH));
	let pre_epoch = Date::from_date(cx, Utc.timestamp_millis(PRE_EPOCH));

	assert!(epoch.is_valid(cx));
	assert!(post_epoch.is_valid(cx));
	assert!(pre_epoch.is_valid(cx));

	assert_eq!(Some(Utc.timestamp_millis(EPOCH)), epoch.to_date(cx));
	assert_eq!(Some(Utc.timestamp_millis(POST_EPOCH)), post_epoch.to_date(cx));
	assert_eq!(Some(Utc.timestamp_millis(PRE_EPOCH)), pre_epoch.to_date(cx));
}
