/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::objects::object::IonRawObject;
use crate::functions::macros::IonContext;
use chrono::offset::Utc;
use chrono::DateTime;
use mozjs::jsapi::*;
use mozjs::error::throw_type_error;
use mozjs::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use mozjs::rust::{HandleValue, MutableHandleValue, maybe_wrap_object_value};
use mozjs::jsval::ObjectValue;
use ::std::result::Result;

pub struct IonDate {
	obj: IonRawObject
}

impl IonDate {
	unsafe fn new(cx: IonContext) -> IonDate {
		IonDate::from(cx, NewDateObject(cx, ClippedTime {t: Utc::now().timestamp_millis() as f64})).unwrap()
	}

	unsafe fn from_date(cx: IonContext, time: DateTime<Utc>) -> IonDate {
		IonDate::from(cx, NewDateObject(cx, ClippedTime {t: time.timestamp_millis() as f64})).unwrap()
	}

	unsafe fn from(cx: IonContext, obj: IonRawObject) -> Option<IonDate> {
		rooted!(in(cx) let mut robj = obj);
		let mut is_date = false;

		if ObjectIsDate(cx, robj.handle_mut().into(), &mut is_date) || !is_date {
			throw_type_error(cx, "Object cannot be converted to Date");
			None
		} else {
			Some(IonDate {obj})
		}
	}

	unsafe fn from_value(cx: IonContext, val: Value) -> Option<IonDate> {
		assert!(val.is_object());
		IonDate::from(cx,val.to_object())
	}

	unsafe fn raw(&self) -> IonRawObject {
		self.obj
	}

	unsafe fn is_valid(&self, cx: IonContext) -> bool {
		rooted!(in(cx) let obj = self.obj);
		let mut is_valid = true;
		return DateIsValid(cx, obj.handle().into(), &mut is_valid) && is_valid;
	}
}


impl FromJSValConvertible for IonDate {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: IonContext, value: HandleValue, _option: ()) -> Result<ConversionResult<IonDate>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "Value is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		if let Some(date) = IonDate::from(cx, value.to_object()) {
			Ok(ConversionResult::Success(date))
		} else {
			Err(())
		}
	}
}

impl ToJSValConvertible for IonDate {
	#[inline]
	unsafe fn to_jsval(&self, cx: IonContext, mut rval: MutableHandleValue) {
		rval.set(ObjectValue(self.raw()));
		maybe_wrap_object_value(cx, rval);
	}
}
