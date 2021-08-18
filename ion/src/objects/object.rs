/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;
use std::result::Result;

use mozjs::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use mozjs::error::throw_type_error;
use mozjs::jsapi::{JSObject, JSTracer, PropertyKey, Value};
use mozjs::jsapi::{
	AssertSameCompartment, GetPropertyKeys, JS_DefineFunction, JS_DefineProperty, JS_DeleteProperty1, JS_GetProperty, JS_HasOwnProperty,
	JS_HasProperty, JS_NewPlainObject, JS_SetProperty,
};
use mozjs::jsapi::{JSITER_HIDDEN, JSITER_OWNONLY, JSITER_SYMBOLS, JSPROP_ENUMERATE, JSPROP_PERMANENT, JSPROP_READONLY};
use mozjs::jsval::{ObjectValue, UndefinedValue};
use mozjs::rust::{CustomTrace, GCMethods, HandleValue, IdVector, maybe_wrap_object_value, MutableHandleValue};
use mozjs_sys::jsgc::RootKind;

use crate::exception::Exception;
use crate::functions::function::{IonFunction, IonNativeFunction};
use crate::IonContext;
use crate::types::values::from_value;

pub const JSPROP_CONSTANT: u16 = (JSPROP_READONLY | JSPROP_ENUMERATE | JSPROP_PERMANENT) as u16;

pub type IonRawObject = *mut JSObject;

#[derive(Clone, Copy, Debug)]
pub struct IonObject {
	obj: IonRawObject,
}

impl IonObject {
	pub fn raw(&self) -> IonRawObject {
		self.obj
	}

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

	pub unsafe fn to_value(&self) -> Value {
		ObjectValue(self.obj)
	}

	pub unsafe fn has(&self, cx: IonContext, key: String) -> bool {
		let key = format!("{}\0", key);
		let mut found = false;
		rooted!(in(cx) let obj = self.obj);

		if JS_HasProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, &mut found) {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	pub unsafe fn has_own(&self, cx: IonContext, key: String) -> bool {
		let key = format!("{}\0", key);
		let mut found = false;
		rooted!(in(cx) let obj = self.obj);

		if JS_HasOwnProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, &mut found) {
			found
		} else {
			Exception::clear(cx);
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

	pub unsafe fn get_as<T: FromJSValConvertible>(&self, cx: IonContext, key: String, config: T::Config) -> Option<T> {
		let opt = self.get(cx, key);
		if let Some(val) = opt {
			from_value(cx, val, config)
		} else {
			None
		}
	}

	pub unsafe fn set(&mut self, cx: IonContext, key: String, value: Value) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let obj = self.obj);
		rooted!(in(cx) let rval = value);
		JS_SetProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, rval.handle().into())
	}

	pub unsafe fn set_as<T: ToJSValConvertible + RootKind + GCMethods>(&mut self, cx: IonContext, key: String, value: T) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let mut val = UndefinedValue());
		value.to_jsval(cx, val.handle_mut());
		self.set(cx, key, val.get())
	}

	pub unsafe fn define(&mut self, cx: IonContext, key: String, value: Value, attrs: u32) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let obj = self.obj);
		rooted!(in(cx) let rval = value);
		JS_DefineProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, rval.handle().into(), attrs)
	}

	pub unsafe fn define_as<T: ToJSValConvertible + RootKind + GCMethods>(&mut self, cx: IonContext, key: String, value: T, attrs: u32) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let mut val = UndefinedValue());
		value.to_jsval(cx, val.handle_mut());
		self.define(cx, key, val.get(), attrs)
	}

	pub unsafe fn define_method(&mut self, cx: IonContext, name: String, method: IonNativeFunction, nargs: u32, attrs: u32) -> IonFunction {
		let name = format!("{}\0", name);
		rooted!(in(cx) let mut obj = self.obj);
		IonFunction::from(JS_DefineFunction(
			cx,
			obj.handle().into(),
			name.as_ptr() as *const i8,
			Some(method),
			nargs,
			attrs,
		))
	}

	pub unsafe fn delete(&self, cx: IonContext, key: String) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let obj = self.obj);
		JS_DeleteProperty1(cx, obj.handle().into(), key.as_ptr() as *const i8)
	}

	// TODO: Return Vec<String> - Waiting on rust-mozjs #544
	pub unsafe fn keys(&mut self, cx: IonContext) -> Vec<PropertyKey> {
		let mut ids = IdVector::new(cx);
		rooted!(in(cx) let obj = self.obj);
		GetPropertyKeys(cx, obj.handle().into(), JSITER_OWNONLY | JSITER_HIDDEN | JSITER_SYMBOLS, ids.handle_mut());
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

impl Deref for IonObject {
	type Target = IonRawObject;

	fn deref(&self) -> &Self::Target {
		&self.obj
	}
}

unsafe impl CustomTrace for IonObject {
	fn trace(&self, tracer: *mut JSTracer) {
		self.obj.trace(tracer)
	}
}
