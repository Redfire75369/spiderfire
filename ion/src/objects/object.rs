/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::*;
use mozjs::conversions::{ToJSValConvertible, FromJSValConvertible, ConversionResult};
use mozjs::rust::{MutableHandleValue, maybe_wrap_object_value, HandleValue, IdVector};
use mozjs::jsval::{ObjectValue};
use crate::functions::macros::IonContext;
use mozjs::error::throw_type_error;
use ::std::result::Result;

pub type IonRawObject = *mut JSObject;

pub struct IonObject {
	obj: IonRawObject
}

impl IonObject {
	unsafe fn new(cx: IonContext) -> IonObject {
		IonObject::from(JS_NewPlainObject(cx))
	}

	unsafe fn from(obj: IonRawObject) -> IonObject {
		IonObject {
			obj
		}
	}

	unsafe fn from_value(val: Value) -> IonObject {
		assert!(val.is_object());
		IonObject::from(val.to_object())
	}

	unsafe fn raw(&self) -> IonRawObject {
		self.obj
	}

	unsafe fn has(&self, cx: IonContext, key: String) -> bool {
		rooted!(in(cx) let obj = self.obj);
		let mut found = false;

		if JS_HasProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, &mut found) {
			found
		} else {
			JS_ClearPendingException(cx);
			false
		}
	}

	unsafe fn get(&self, cx: IonContext, key: String) -> Option<Value> {
		if self.has(cx, key.clone()) {
			rooted!(in(cx) let obj = self.obj);
			rooted!(in(cx) let mut rval: Value);
			JS_GetProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, rval.handle_mut().into());
			Some(rval.get())
		} else {
			None
		}
	}

	unsafe fn set(&mut self, cx: IonContext, key: String, value: Value) -> bool {
		rooted!(in(cx) let mut obj = self.obj);
		rooted!(in(cx) let rval = value);
		JS_SetProperty(cx, obj.handle_mut().into(), key.as_ptr() as *const i8, rval.handle().into())
	}

	unsafe fn keys(&mut self, cx: IonContext) -> Vec<PropertyKey> {
		rooted!(in(cx) let mut obj = self.obj);
		let mut ids = IdVector::new(cx);
		GetPropertyKeys(cx, obj.handle().into(), JSITER_OWNONLY | JSITER_HIDDEN | JSITER_SYMBOLS, ids.handle_mut().into());
		ids.to_vec()
	}
}

impl FromJSValConvertible for IonObject {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: IonContext, value: HandleValue, _option: ()) -> Result<ConversionResult<IonObject>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "Value is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		Ok(ConversionResult::Success(IonObject::from(value.to_object())))
	}
}

impl ToJSValConvertible for IonObject {
	#[inline]
	unsafe fn to_jsval(&self, cx: IonContext, mut rval: MutableHandleValue) {
		rval.set(ObjectValue(self.raw()));
		maybe_wrap_object_value(cx, rval);
	}
}
