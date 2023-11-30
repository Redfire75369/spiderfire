/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use mozjs::glue::{SetAccessorPropertyDescriptor, SetDataPropertyDescriptor};
use mozjs::jsapi::{FromPropertyDescriptor, Heap, ObjectToCompletePropertyDescriptor};
use mozjs::jsapi::PropertyDescriptor as JSPropertyDescriptor;

use crate::{Context, Function, Object, Root, Value};
use crate::flags::PropertyFlags;

pub struct PropertyDescriptor {
	desc: Root<Box<Heap<JSPropertyDescriptor>>>,
}

impl PropertyDescriptor {
	pub fn new(cx: &Context, value: &Value, attrs: PropertyFlags) -> PropertyDescriptor {
		let mut desc = PropertyDescriptor::from(cx.root_property_descriptor(JSPropertyDescriptor::default()));
		unsafe { SetDataPropertyDescriptor(desc.handle_mut().into(), value.handle().into(), attrs.bits() as u32) };
		desc
	}

	pub fn new_accessor(
		cx: &Context, getter: &Function, setter: &Function, attrs: PropertyFlags,
	) -> PropertyDescriptor {
		let getter = getter.to_object(cx);
		let setter = setter.to_object(cx);
		let mut desc = PropertyDescriptor::from(cx.root_property_descriptor(JSPropertyDescriptor::default()));
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

	pub fn from_object(cx: &Context, object: &Object) -> Option<PropertyDescriptor> {
		let mut desc = PropertyDescriptor::from(cx.root_property_descriptor(JSPropertyDescriptor::default()));
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

	pub fn to_object(&self, cx: &Context) -> Option<Object> {
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

	pub fn getter(&self, cx: &Context) -> Option<Object> {
		self.handle().hasGetter_().then(|| Object::from(cx.root_object(self.handle().getter_)))
	}

	pub fn setter(&self, cx: &Context) -> Option<Object> {
		self.handle().hasSetter_().then(|| Object::from(cx.root_object(self.handle().setter_)))
	}

	pub fn value(&self, cx: &Context) -> Option<Value> {
		self.handle().hasValue_().then(|| Value::from(cx.root_value(self.handle().value_)))
	}

	pub fn into_root(self) -> Root<Box<Heap<JSPropertyDescriptor>>> {
		self.desc
	}
}

impl From<Root<Box<Heap<JSPropertyDescriptor>>>> for PropertyDescriptor {
	fn from(desc: Root<Box<Heap<JSPropertyDescriptor>>>) -> PropertyDescriptor {
		PropertyDescriptor { desc }
	}
}

impl Deref for PropertyDescriptor {
	type Target = Root<Box<Heap<JSPropertyDescriptor>>>;

	fn deref(&self) -> &Self::Target {
		&self.desc
	}
}

impl DerefMut for PropertyDescriptor {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.desc
	}
}
