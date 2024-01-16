/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use mozjs::glue::{SetAccessorPropertyDescriptor, SetDataPropertyDescriptor};
use mozjs::jsapi::{FromPropertyDescriptor, JS_GetObjectFunction, ObjectToCompletePropertyDescriptor};
use mozjs::jsapi::PropertyDescriptor as JSPropertyDescriptor;

use crate::{Context, Function, Local, Object, Value};
use crate::flags::PropertyFlags;

pub struct PropertyDescriptor<'pd> {
	desc: Local<'pd, JSPropertyDescriptor>,
}

impl<'pd> PropertyDescriptor<'pd> {
	pub fn empty(cx: &'pd Context) -> PropertyDescriptor<'pd> {
		PropertyDescriptor::from(cx.root_property_descriptor(JSPropertyDescriptor::default()))
	}

	pub fn new(cx: &'pd Context, value: &Value, attrs: PropertyFlags) -> PropertyDescriptor<'pd> {
		let mut desc = PropertyDescriptor::empty(cx);
		unsafe { SetDataPropertyDescriptor(desc.handle_mut().into(), value.handle().into(), attrs.bits() as u32) };
		desc
	}

	pub fn new_accessor(
		cx: &'pd Context, getter: &Function, setter: &Function, attrs: PropertyFlags,
	) -> PropertyDescriptor<'pd> {
		let getter = getter.to_object(cx);
		let setter = setter.to_object(cx);
		let mut desc = PropertyDescriptor::empty(cx);
		unsafe {
			SetAccessorPropertyDescriptor(
				desc.handle_mut().into(),
				getter.handle().into(),
				setter.handle().into(),
				attrs.bits() as u32,
			)
		};
		desc
	}

	pub fn from_object(cx: &'pd Context, object: &Object) -> Option<PropertyDescriptor<'pd>> {
		let mut desc = PropertyDescriptor::empty(cx);
		let desc_value = Value::object(cx, object);
		unsafe {
			ObjectToCompletePropertyDescriptor(
				cx.as_ptr(),
				object.handle().into(),
				desc_value.handle().into(),
				desc.handle_mut().into(),
			)
			.then_some(desc)
		}
	}

	pub fn to_object<'cx>(&self, cx: &'cx Context) -> Option<Object<'cx>> {
		let mut value = Value::undefined(cx);
		unsafe { FromPropertyDescriptor(cx.as_ptr(), self.handle().into(), value.handle_mut().into()) }
			.then(|| value.to_object(cx))
	}

	pub fn is_configurable(&self) -> bool {
		self.handle().hasConfigurable_() && self.handle().configurable_()
	}

	pub fn is_enumerable(&self) -> bool {
		self.handle().hasEnumerable_() && self.handle().enumerable_()
	}

	pub fn is_writable(&self) -> bool {
		self.handle().hasWritable_() && self.handle().writable_()
	}

	pub fn is_resolving(&self) -> bool {
		self.handle().resolving_()
	}

	pub fn getter<'cx>(&self, cx: &'cx Context) -> Option<Function<'cx>> {
		if self.handle().hasGetter_() && !self.handle().getter_.is_null() {
			Some(Function::from(
				cx.root_function(unsafe { JS_GetObjectFunction(self.handle().getter_) }),
			))
		} else {
			None
		}
	}

	pub fn setter<'cx>(&self, cx: &'cx Context) -> Option<Function<'cx>> {
		if self.handle().hasSetter_() && !self.handle().setter_.is_null() {
			Some(Function::from(
				cx.root_function(unsafe { JS_GetObjectFunction(self.handle().setter_) }),
			))
		} else {
			None
		}
	}

	pub fn value<'cx>(&self, cx: &'cx Context) -> Option<Value<'cx>> {
		self.handle().hasValue_().then(|| Value::from(cx.root_value(self.handle().value_)))
	}

	pub fn into_local(self) -> Local<'pd, JSPropertyDescriptor> {
		self.desc
	}
}

impl<'pd> From<Local<'pd, JSPropertyDescriptor>> for PropertyDescriptor<'pd> {
	fn from(desc: Local<'pd, JSPropertyDescriptor>) -> PropertyDescriptor<'pd> {
		PropertyDescriptor { desc }
	}
}

impl<'pd> Deref for PropertyDescriptor<'pd> {
	type Target = Local<'pd, JSPropertyDescriptor>;

	fn deref(&self) -> &Self::Target {
		&self.desc
	}
}

impl<'pd> DerefMut for PropertyDescriptor<'pd> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.desc
	}
}
