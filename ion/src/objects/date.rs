/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use chrono::{DateTime, TimeZone};
use chrono::offset::Utc;
use mozjs::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use mozjs::error::throw_type_error;
use mozjs::jsapi::{AssertSameCompartment, ClippedTime, DateGetMsecSinceEpoch, DateIsValid, JSObject, JSTracer, NewDateObject, ObjectIsDate};
use mozjs::jsval::{JSVal, ObjectValue};
use mozjs::rust::{CustomTrace, HandleValue, maybe_wrap_object_value, MutableHandleValue};

use crate::Context;

#[derive(Clone, Copy, Debug)]
pub struct Date {
	obj: *mut JSObject,
}

impl Date {
	/// Creates a new [Date] with the current time.
	pub fn new(cx: Context) -> Date {
		Date::from_date(cx, Utc::now())
	}

	/// Creates a new [Date] with the given time.
	pub fn from_date(cx: Context, time: DateTime<Utc>) -> Date {
		Date::from(cx, unsafe { NewDateObject(cx, ClippedTime { t: time.timestamp_millis() as f64 }) }).unwrap()
	}

	/// Creates a [Date] from a [*mut JSObject].
	pub fn from(cx: Context, obj: *mut JSObject) -> Option<Date> {
		if Date::is_date_raw(cx, obj) {
			Some(Date { obj })
		} else {
			None
		}
	}

	/// Creates a [Date] from a [JSVal].
	pub fn from_value(cx: Context, val: JSVal) -> Option<Date> {
		if val.is_object() {
			Date::from(cx, val.to_object())
		} else {
			None
		}
	}

	/// Converts a [Date] to a [JSVal].
	pub fn to_value(&self) -> JSVal {
		ObjectValue(self.obj)
	}

	/// Checks if the [Date] is a valid date.
	pub fn is_valid(&self, cx: Context) -> bool {
		rooted!(in(cx) let obj = self.obj);
		let mut is_valid = true;
		(unsafe { DateIsValid(cx, obj.handle().into(), &mut is_valid) }) && is_valid
	}

	/// Converts the [Date] to a [DateTime].
	pub fn to_date(&self, cx: Context) -> Option<DateTime<Utc>> {
		rooted!(in(cx) let obj = self.obj);
		let mut milliseconds: f64 = f64::MAX;
		if !unsafe { DateGetMsecSinceEpoch(cx, obj.handle().into(), &mut milliseconds) } || milliseconds == f64::MAX {
			None
		} else {
			Some(Utc.timestamp_millis(milliseconds as i64))
		}
	}

	/// Checks if a [*mut JSObject] is a date.
	pub fn is_date_raw(cx: Context, obj: *mut JSObject) -> bool {
		rooted!(in(cx) let mut robj = obj);
		let mut is_date = false;
		(unsafe { ObjectIsDate(cx, robj.handle_mut().into(), &mut is_date) }) && is_date
	}
}

impl FromJSValConvertible for Date {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: Context, value: HandleValue, _: ()) -> Result<ConversionResult<Date>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "JSVal is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		if let Some(date) = Date::from(cx, value.to_object()) {
			Ok(ConversionResult::Success(date))
		} else {
			Err(())
		}
	}
}

impl ToJSValConvertible for Date {
	#[inline]
	unsafe fn to_jsval(&self, cx: Context, mut rval: MutableHandleValue) {
		rval.set(self.to_value());
		maybe_wrap_object_value(cx, rval);
	}
}

impl Deref for Date {
	type Target = *mut JSObject;

	fn deref(&self) -> &Self::Target {
		&self.obj
	}
}

unsafe impl CustomTrace for Date {
	fn trace(&self, tracer: *mut JSTracer) {
		self.obj.trace(tracer)
	}
}
