/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::result::Result;

use mozjs::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use mozjs::conversions::ConversionResult::Success;
use mozjs::error::throw_type_error;
use mozjs::jsapi::*;
use mozjs::jsval::ObjectValue;
use mozjs::rust::{HandleValue, IdVector, maybe_wrap_object_value, MutableHandleValue};

use crate::functions::macros::IonContext;

pub type IonRawObject = *mut JSObject;

pub struct IonObject {
	obj: IonRawObject,
}

impl IonObject {
	#[allow(dead_code)]
	pub unsafe fn new(cx: IonContext) -> IonObject {
		IonObject::from(JS_NewPlainObject(cx))
	}

	pub unsafe fn from(obj: IonRawObject) -> IonObject {
		IonObject { obj }
	}

	pub unsafe fn from_value(val: Value) -> IonObject {
		assert!(val.is_object());
		IonObject::from(val.to_object())
	}

	pub unsafe fn raw(&self) -> IonRawObject {
		self.obj
	}

	pub unsafe fn to_value(&self) -> Value {
		ObjectValue(self.raw())
	}

	#[allow(dead_code)]
	pub unsafe fn has(&self, cx: IonContext, key: String) -> bool {
		let key = format!("{}\0", key);
		let mut found = false;
		rooted!(in(cx) let obj = self.obj);

		if JS_HasProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, &mut found) {
			found
		} else {
			JS_ClearPendingException(cx);
			false
		}
	}

	pub unsafe fn get(&self, cx: IonContext, key: String) -> Option<Value> {
		let key = format!("{}\0", key);
		if self.has(cx, key.clone()) {
			rooted!(in(cx) let obj = self.obj);
			rooted!(in(cx) let mut rval: Value);
			JS_GetProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, rval.handle_mut().into());
			Some(rval.get())
		} else {
			None
		}
	}

	#[allow(dead_code)]
	pub unsafe fn get_as<T: FromJSValConvertible>(&self, cx: IonContext, key: String, config: T::Config) -> Option<T> {
		let opt = self.get(cx, key);
		if let Some(val) = opt {
			rooted!(in(cx) let rooted_val = val);
			if let Success(v) = T::from_jsval(cx, rooted_val.handle(), config).unwrap() {
				Some(v)
			} else {
				None
			}
		} else {
			None
		}
	}

	#[allow(dead_code)]
	pub unsafe fn set(&mut self, cx: IonContext, key: String, value: Value) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let obj = self.obj);
		rooted!(in(cx) let rval = value);
		JS_SetProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, rval.handle().into())
	}

	pub unsafe fn define(&mut self, cx: IonContext, key: String, value: Value, attrs: u32) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let obj = self.obj);
		rooted!(in(cx) let rval = value);
		JS_DefineProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, rval.handle().into(), attrs)
	}

	#[allow(dead_code)]
	pub unsafe fn delete(&self, cx: IonContext, key: String) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let obj = self.obj);
		JS_DeleteProperty1(cx, obj.handle().into(), key.as_ptr() as *const i8)
	}

	// Waiting on rust-mozjs #546
	// pub unsafe fn set_as<T: ToJSValConvertible + GCMethods>(&mut self, cx: IonContext, key: String, value: T) -> bool {
	// 	rooted!(in(cx) let mut val = value);
	// 	value.to_jsval(cx, val.handle_mut());
	// 	self.set(cx, key, val.get())
	// }

	// TODO: Return Vec<String> - Waiting on rust-mozjs #544
	#[allow(dead_code)]
	pub unsafe fn keys(&mut self, cx: IonContext) -> Vec<PropertyKey> {
		let mut ids = IdVector::new(cx);
		rooted!(in(cx) let obj = self.obj);
		GetPropertyKeys(
			cx,
			obj.handle().into(),
			JSITER_OWNONLY | JSITER_HIDDEN | JSITER_SYMBOLS,
			ids.handle_mut().into(),
		);
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
