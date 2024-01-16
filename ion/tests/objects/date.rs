use chrono::{TimeZone, Utc};
use mozjs::jsapi::JSAutoRealm;
use mozjs::rust::{JSEngine, Runtime};

use ion::{Context, Date};
use ion::object::default_new_global;

const EPOCH: i64 = 0; // 01 January 1970
const POST_EPOCH: i64 = 1615766400; // 15 March 2021
const PRE_EPOCH: i64 = -1615766400; // 20 October 1918

#[test]
fn date() {
	let engine = JSEngine::init().unwrap();
	let runtime = Runtime::new(engine.handle());

	let cx = &Context::from_runtime(&runtime);
	let global = default_new_global(cx);
	let _realm = JSAutoRealm::new(runtime.cx(), global.handle().get());

	let epoch = Date::from_date(cx, Utc.timestamp_millis_opt(EPOCH).unwrap());
	let post_epoch = Date::from_date(cx, Utc.timestamp_millis_opt(POST_EPOCH).unwrap());
	let pre_epoch = Date::from_date(cx, Utc.timestamp_millis_opt(PRE_EPOCH).unwrap());

	assert!(epoch.is_valid(cx));
	assert!(post_epoch.is_valid(cx));
	assert!(pre_epoch.is_valid(cx));

	assert_eq!(Some(Utc.timestamp_millis_opt(EPOCH).unwrap()), epoch.to_date(cx));
	assert_eq!(
		Some(Utc.timestamp_millis_opt(POST_EPOCH).unwrap()),
		post_epoch.to_date(cx)
	);
	assert_eq!(
		Some(Utc.timestamp_millis_opt(PRE_EPOCH).unwrap()),
		pre_epoch.to_date(cx)
	);
}
