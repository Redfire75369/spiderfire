use chrono::{TimeZone, Utc};

use ion::utils::test::TestRuntime;
use ion::Date;

const EPOCH: i64 = 0; // 01 January 1970
const POST_EPOCH: i64 = 1615766400; // 15 March 2021
const PRE_EPOCH: i64 = -1615766400; // 20 October 1918

#[test]
fn date() {
	let rt = TestRuntime::new();
	let cx = &rt.cx;

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
