/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use chrono::{DateTime, TimeZone};
use chrono::offset::Utc;
use mozjs::jsapi::{ClippedTime, DateGetMsecSinceEpoch, DateIsValid, JSObject, NewDateObject, ObjectIsDate};
use mozjs::jsval::{JSVal, ObjectValue};
use mozjs::rust::{Handle, MutableHandle};

use crate::{Context, Local};

#[derive(Debug)]
pub struct Date<'cx> {
	date: &'cx mut Local<'cx, *mut JSObject>,
}

impl<'cx> Date<'cx> {
	/// Creates a new Date Object with the current time.
	pub fn new(cx: &'cx Context) -> Date<'cx> {
		Date::from_date(cx, Utc::now())
	}

	/// Creates a new Date Object with the given time.
	pub fn from_date(cx: &'cx Context, time: DateTime<Utc>) -> Date<'cx> {
		Date {
			date: cx.root_object(unsafe { NewDateObject(**cx, ClippedTime { t: time.timestamp_millis() as f64 }) }),
		}
	}

	/// Creates a [Date] from an object.
	pub fn from(cx: &'cx Context, object: &'cx mut Local<'cx, *mut JSObject>) -> Option<Date<'cx>> {
		if Date::is_date(cx, &object) {
			Some(Date { date: object })
		} else {
			None
		}
	}

	/// Converts a [Date] to a [JSVal].
	pub fn to_value(&self) -> JSVal {
		ObjectValue(**self.date)
	}

	/// Checks if the [Date] is a valid date.
	pub fn is_valid(&self, cx: &Context) -> bool {
		let mut is_valid = true;
		(unsafe { DateIsValid(**cx, self.date.handle().into(), &mut is_valid) }) && is_valid
	}

	/// Converts the [Date] to a [DateTime].
	pub fn to_date(&self, cx: &Context) -> Option<DateTime<Utc>> {
		let mut milliseconds: f64 = f64::MAX;
		if !unsafe { DateGetMsecSinceEpoch(**cx, self.date.handle().into(), &mut milliseconds) } || milliseconds == f64::MAX {
			None
		} else {
			Utc.timestamp_millis_opt(milliseconds as i64).single()
		}
	}

	pub fn handle<'a>(&'a self) -> Handle<'a, *mut JSObject>
	where
		'cx: 'a,
	{
		self.date.handle()
	}

	pub fn handle_mut<'a>(&'a mut self) -> MutableHandle<'a, *mut JSObject>
	where
		'cx: 'a,
	{
		self.date.handle_mut()
	}

	/// Checks if a [*mut JSObject] is a date.
	pub fn is_date_raw(cx: &Context, object: *mut JSObject) -> bool {
		rooted!(in(**cx) let object = object);
		let mut is_date = false;
		(unsafe { ObjectIsDate(**cx, object.handle().into(), &mut is_date) }) && is_date
	}

	/// Checks if a [*mut JSObject] is a date.
	pub fn is_date(cx: &Context, object: &Local<*mut JSObject>) -> bool {
		let mut is_date = false;
		(unsafe { ObjectIsDate(**cx, object.handle().into(), &mut is_date) }) && is_date
	}
}

impl<'cx> Deref for Date<'cx> {
	type Target = Local<'cx, *mut JSObject>;

	fn deref(&self) -> &Self::Target {
		&*self.date
	}
}
