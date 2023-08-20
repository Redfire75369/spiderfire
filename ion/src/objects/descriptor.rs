/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use mozjs::glue::SetDataPropertyDescriptor;
use mozjs::jsapi::{FromPropertyDescriptor, ObjectToCompletePropertyDescriptor};
use mozjs::jsapi::PropertyDescriptor as JSPropertyDescriptor;
use mozjs::rust::{Handle, MutableHandle};

use crate::{Context, Local, Object, Value};
use crate::flags::PropertyFlags;

pub struct PropertyDescriptor<'pd> {
	desc: Local<'pd, JSPropertyDescriptor>,
}

impl<'pd> PropertyDescriptor<'pd> {
	pub fn new(cx: &'pd Context, value: &Value, attrs: PropertyFlags) -> PropertyDescriptor<'pd> {
		let mut desc = PropertyDescriptor::from(cx.root_property_descriptor(JSPropertyDescriptor::default()));
		unsafe { SetDataPropertyDescriptor(desc.handle_mut().into(), value.handle().into(), attrs.bits() as u32) };
		desc
	}

	pub fn from_object(cx: &'pd Context, object: &Object) -> Option<PropertyDescriptor<'pd>> {
		let mut desc = PropertyDescriptor::from(cx.root_property_descriptor(JSPropertyDescriptor::default()));
		let desc_value = Value::object(cx, object);
		unsafe { ObjectToCompletePropertyDescriptor(cx.as_ptr(), object.handle().into(), desc_value.handle().into(), desc.handle_mut().into()) }
			.then_some(desc)
	}

	pub fn to_object<'cx>(&self, cx: &'cx Context) -> Option<Object<'cx>> {
		let mut value = Value::undefined(cx);
		unsafe { FromPropertyDescriptor(cx.as_ptr(), self.handle().into(), value.handle_mut().into()) }.then(|| value.to_object(cx))
	}

	pub fn is_configurable(&self) -> bool {
		self.desc.hasConfigurable_() && self.desc.configurable_()
	}

	pub fn is_enumerable(&self) -> bool {
		self.desc.hasEnumerable_() && self.desc.enumerable_()
	}

	pub fn is_writable(&self) -> bool {
		self.desc.hasWritable_() && self.desc.writable_()
	}

	pub fn is_resolving(&self) -> bool {
		self.desc.resolving_()
	}

	pub fn getter<'cx>(&self, cx: &'cx Context) -> Option<Object<'cx>> {
		self.hasGetter_().then(|| Object::from(cx.root_object(self.getter_)))
	}

	pub fn setter<'cx>(&self, cx: &'cx Context) -> Option<Object<'cx>> {
		self.hasSetter_().then(|| Object::from(cx.root_object(self.setter_)))
	}

	pub fn value<'cx>(&self, cx: &'cx Context) -> Option<Value<'cx>> {
		self.hasValue_().then(|| Value::from(cx.root_value(self.value_)))
	}

	pub fn handle<'s>(&'s self) -> Handle<'s, JSPropertyDescriptor>
	where
		'pd: 's,
	{
		self.desc.handle()
	}

	pub fn handle_mut<'s>(&'s mut self) -> MutableHandle<'s, JSPropertyDescriptor>
	where
		'pd: 's,
	{
		self.desc.handle_mut()
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

	fn deref(&self) -> &Local<'pd, JSPropertyDescriptor> {
		&self.desc
	}
}
