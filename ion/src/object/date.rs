/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use chrono::{DateTime, TimeZone};
use chrono::offset::Utc;
use mozjs::jsapi::{ClippedTime, DateGetMsecSinceEpoch, DateIsValid, JSObject, NewDateObject, ObjectIsDate};

use crate::{Context, Local};

/// Represents a `Date` in the JavaScript Runtime.
/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date) for more details.
#[derive(Debug)]
pub struct Date<'d> {
	date: Local<'d, *mut JSObject>,
}

impl<'d> Date<'d> {
	/// Creates a new [Date] with the current time.
	pub fn new(cx: &'d Context) -> Date<'d> {
		Date::from_date(cx, Utc::now())
	}

	/// Creates a new [Date] with the given time.
	pub fn from_date(cx: &'d Context, time: DateTime<Utc>) -> Date<'d> {
		let date = unsafe { NewDateObject(cx.as_ptr(), ClippedTime { t: time.timestamp_millis() as f64 }) };
		Date { date: cx.root_object(date) }
	}

	/// Creates a [Date] from an object.
	/// Returns [None] if it is not a [Date].
	pub fn from(cx: &Context, object: Local<'d, *mut JSObject>) -> Option<Date<'d>> {
		if Date::is_date(cx, &object) {
			Some(Date { date: object })
		} else {
			None
		}
	}

	/// Creates a [Date] from an object.
	///
	/// ### Safety
	/// Object must be a [Date].
	pub unsafe fn from_unchecked(object: Local<'d, *mut JSObject>) -> Date<'d> {
		Date { date: object }
	}

	/// Checks if the [Date] is a valid date.
	pub fn is_valid(&self, cx: &Context) -> bool {
		let mut is_valid = true;
		(unsafe { DateIsValid(cx.as_ptr(), self.date.handle().into(), &mut is_valid) }) && is_valid
	}

	/// Converts the [Date] to a [DateTime].
	pub fn to_date(&self, cx: &Context) -> Option<DateTime<Utc>> {
		let mut milliseconds: f64 = f64::MAX;
		if !unsafe { DateGetMsecSinceEpoch(cx.as_ptr(), self.date.handle().into(), &mut milliseconds) }
			|| milliseconds == f64::MAX
		{
			None
		} else {
			Utc.timestamp_millis_opt(milliseconds as i64).single()
		}
	}

	/// Checks if a [raw object](*mut JSObject) is a date.
	pub fn is_date_raw(cx: &Context, object: *mut JSObject) -> bool {
		rooted!(in(cx.as_ptr()) let object = object);
		let mut is_date = false;
		(unsafe { ObjectIsDate(cx.as_ptr(), object.handle().into(), &mut is_date) }) && is_date
	}

	/// Checks if an object is a date.
	pub fn is_date(cx: &Context, object: &Local<*mut JSObject>) -> bool {
		let mut is_date = false;
		(unsafe { ObjectIsDate(cx.as_ptr(), object.handle().into(), &mut is_date) }) && is_date
	}
}

impl<'d> Deref for Date<'d> {
	type Target = Local<'d, *mut JSObject>;

	fn deref(&self) -> &Self::Target {
		&self.date
	}
}

impl<'d> DerefMut for Date<'d> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.date
	}
}
