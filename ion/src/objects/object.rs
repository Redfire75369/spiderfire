/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};

use mozjs::jsapi::{
	CurrentGlobalOrNull, GetPropertyKeys, JS_DefineFunctionById, JS_DefineFunctions, JS_DefineFunctionsWithHelp, JS_DefinePropertyById2,
	JS_DeletePropertyById, JS_GetPropertyById, JS_HasOwnPropertyById, JS_HasPropertyById, JS_NewPlainObject, JS_SetPropertyById, JSFunctionSpec,
	JSFunctionSpecWithHelp, JSObject,
};
use mozjs::jsval::NullValue;
use mozjs::rust::{Handle, IdVector, MutableHandle};

use crate::{Context, Exception, Function, Local, PropertyKey, Value};
use crate::conversions::{FromValue, ToKey, ToValue};
use crate::flags::{IteratorFlags, PropertyFlags};
use crate::functions::NativeFunction;

/// Represents an [Object] in the JS Runtime.
///
/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object) for more details.
#[derive(Debug)]
pub struct Object<'o> {
	object: Local<'o, *mut JSObject>,
}

impl<'o> Object<'o> {
	/// Creates a plain empty [Object].
	pub fn new<'cx>(cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(unsafe { JS_NewPlainObject(**cx) }))
	}

	/// Creates a `null` "Object".
	///
	/// Most operations on this will result in an error, so be wary of where it is used.
	pub fn null<'cx>(cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(NullValue().to_object_or_null()))
	}

	/// Returns the current global object or `null` if one has not been initialised yet.
	pub fn global<'cx>(cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(unsafe { CurrentGlobalOrNull(**cx) }))
	}

	/// Checks if the [Object] has a value at the given [key](Key).
	pub fn has<'cx, K: ToKey<'cx>>(&self, cx: &'cx Context, key: K) -> bool {
		let key = key.to_key(cx).unwrap();
		let mut found = false;
		if unsafe { JS_HasPropertyById(**cx, self.handle().into(), key.handle().into(), &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Checks if the [Object] has its own value at the given [key](Key).
	///
	/// An object owns its properties if they are not inherited from a prototype.
	pub fn has_own<'cx, K: ToKey<'cx>>(&self, cx: &'cx Context, key: K) -> bool {
		let key = key.to_key(cx).unwrap();
		let mut found = false;
		if unsafe { JS_HasOwnPropertyById(**cx, self.handle().into(), key.handle().into(), &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Gets the [Value] at the given key of the [Object].
	///
	/// Returns [None] if there is no value at the given [key](Key).
	pub fn get<'cx, K: ToKey<'cx>>(&self, cx: &'cx Context, key: K) -> Option<Value<'cx>> {
		let key = key.to_key(cx).unwrap();
		if self.has(cx, &key) {
			let mut rval = Value::undefined(cx);
			unsafe { JS_GetPropertyById(**cx, self.handle().into(), key.handle().into(), rval.handle_mut().into()) };
			Some(rval)
		} else {
			None
		}
	}

	/// Gets the value at the given key of the [Object]. as a Rust type.
	/// Returns [None] if the object does not contain the [key](Key) or conversion to the Rust type fails.
	pub fn get_as<'cx, K: ToKey<'cx>, T: FromValue<'cx>>(&self, cx: &'cx Context, key: K, strict: bool, config: T::Config) -> Option<T> {
		self.get(cx, key).and_then(|val| unsafe { T::from_value(cx, &val, strict, config).ok() })
	}

	/// Sets the [Value] at the given [key](Key) of the [Object].
	///
	/// Returns `false` if the property cannot be set.
	pub fn set<'cx, K: ToKey<'cx>>(&mut self, cx: &'cx Context, key: K, value: &Value) -> bool {
		let key = key.to_key(cx).unwrap();
		unsafe { JS_SetPropertyById(**cx, self.handle().into(), key.handle().into(), value.handle().into()) }
	}

	/// Sets the Rust type at the given [key](Key) of the [Object].
	///
	/// Returns `false` if the property cannot be set.
	pub fn set_as<'cx, K: ToKey<'cx>, T: ToValue<'cx> + ?Sized>(&mut self, cx: &'cx Context, key: K, value: &T) -> bool {
		self.set(cx, key, unsafe { &value.as_value(cx) })
	}

	/// Defines the [Value] at the given [key](Key) of the [Object] with the given attributes.
	///
	/// Returns `false` if the property cannot be defined.
	pub fn define<'cx, K: ToKey<'cx>>(&mut self, cx: &'cx Context, key: K, value: &Value, attrs: PropertyFlags) -> bool {
		let key = key.to_key(cx).unwrap();
		unsafe {
			JS_DefinePropertyById2(
				**cx,
				self.handle().into(),
				key.handle().into(),
				value.handle().into(),
				attrs.bits() as u32,
			)
		}
	}

	/// Defines the Rust type at the given [key](Key) of the [Object] with the given attributes.
	///
	/// Returns `false` if the property cannot be defined.
	pub fn define_as<'cx, K: ToKey<'cx>, T: ToValue<'cx> + ?Sized>(&mut self, cx: &'cx Context, key: K, value: &T, attrs: PropertyFlags) -> bool {
		self.define(cx, key, unsafe { &value.as_value(cx) }, attrs)
	}

	/// Defines a method with the given name, and the given number of arguments and attributes on the [Object].
	///
	/// Parameters are similar to [create_function_spec](crate::spec::create_function_spec).
	pub fn define_method<'cx, K: ToKey<'cx>>(
		&mut self, cx: &'cx Context, key: K, method: NativeFunction, nargs: u32, attrs: PropertyFlags,
	) -> Function<'cx> {
		let key = key.to_key(cx).unwrap();
		cx.root_function(unsafe { JS_DefineFunctionById(**cx, self.handle().into(), key.handle().into(), Some(method), nargs, attrs.bits() as u32) })
			.into()
	}

	/// Defines methods on the [Object] using the given [JSFunctionSpec]s.
	///
	/// The final element of the `methods` slice must be `JSFunctionSpec::ZERO`.
	///
	/// They can be created through [function_spec](crate::function_spec).
	pub fn define_methods(&mut self, cx: &Context, methods: &[JSFunctionSpec]) -> bool {
		unsafe { JS_DefineFunctions(**cx, self.handle().into(), methods.as_ptr()) }
	}

	/// Defines methods on the [Object] using the given [JSFunctionSpecWithHelp]s.
	///
	/// The final element of the `methods` slice must be `JSFunctionSpecWithHelp::ZERO`.
	pub fn define_methods_with_help(&mut self, cx: &Context, methods: &[JSFunctionSpecWithHelp]) -> bool {
		unsafe { JS_DefineFunctionsWithHelp(**cx, self.handle().into(), methods.as_ptr()) }
	}

	/// Deletes the [Value] at the given index.
	///
	/// Returns `false` if the element cannot be deleted.
	pub fn delete<'cx: 'o, K: ToKey<'cx>>(&self, cx: &'cx Context, key: K) -> bool {
		let key = key.to_key(cx).unwrap();
		let mut result = MaybeUninit::uninit();
		unsafe { JS_DeletePropertyById(**cx, self.handle().into(), key.handle().into(), result.as_mut_ptr()) }
	}

	/// Returns a [Vec] of the keys of the [Object].
	///
	/// Each [Key] can be a [String], [Symbol] or integer.
	pub fn keys<'cx>(&self, cx: &'cx Context, flags: Option<IteratorFlags>) -> Vec<PropertyKey<'cx>> {
		let flags = flags.unwrap_or(IteratorFlags::OWN_ONLY);
		let mut ids = unsafe { IdVector::new(**cx) };
		unsafe { GetPropertyKeys(**cx, self.handle().into(), flags.bits(), ids.handle_mut()) };
		ids.iter().map(|id| id.to_key(cx).unwrap()).collect()
	}

	pub fn iter<'cx: 'o, 's>(&'s self, cx: &'cx Context, flags: Option<IteratorFlags>) -> ObjectIter<'_, 'cx, 's>
	where
		'o: 's,
	{
		let keys = self.keys(cx, flags);
		ObjectIter::new(cx, self, keys)
	}

	pub fn handle<'s>(&'s self) -> Handle<'s, *mut JSObject>
	where
		'o: 's,
	{
		self.object.handle()
	}

	pub fn handle_mut<'s>(&'s mut self) -> MutableHandle<'s, *mut JSObject>
	where
		'o: 's,
	{
		self.object.handle_mut()
	}

	pub fn into_local(self) -> Local<'o, *mut JSObject> {
		self.object
	}
}

impl<'o> From<Local<'o, *mut JSObject>> for Object<'o> {
	fn from(object: Local<'o, *mut JSObject>) -> Object<'o> {
		Object { object }
	}
}

impl<'o> Deref for Object<'o> {
	type Target = Local<'o, *mut JSObject>;

	fn deref(&self) -> &Self::Target {
		&self.object
	}
}

impl<'o> DerefMut for Object<'o> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.object
	}
}

pub struct ObjectIter<'c, 'cx, 'o> {
	cx: &'cx Context<'c>,
	object: &'o Object<'cx>,
	keys: Vec<PropertyKey<'cx>>,
	index: usize,
	count: usize,
}

impl<'c, 'cx, 'o> ObjectIter<'c, 'cx, 'o> {
	fn new(cx: &'cx Context<'c>, object: &'o Object<'cx>, keys: Vec<PropertyKey<'cx>>) -> ObjectIter<'c, 'cx, 'o> {
		let count = keys.len();
		ObjectIter { cx, object, keys, index: 0, count }
	}
}

impl<'c, 'cx, 'o> Iterator for ObjectIter<'c, 'cx, 'o> {
	type Item = (PropertyKey<'cx>, Value<'cx>);

	fn next(&mut self) -> Option<Self::Item> {
		if self.index < self.count {
			let key = &self.keys[self.index];
			self.index += 1;
			Some((key.to_key(self.cx).unwrap(), self.object.get(self.cx, key).unwrap()))
		} else {
			None
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.count, Some(self.count))
	}
}

impl<'c, 'cx, 'o> DoubleEndedIterator for ObjectIter<'c, 'cx, 'o> {
	fn next_back(&mut self) -> Option<Self::Item> {
		if self.index < self.count {
			self.count -= 1;
			let key = &self.keys[self.count];
			Some((key.to_key(self.cx).unwrap(), self.object.get(self.cx, key).unwrap()))
		} else {
			None
		}
	}
}

impl<'c, 'cx, 'o> ExactSizeIterator for ObjectIter<'c, 'cx, 'o> {
	fn len(&self) -> usize {
		self.count - self.index
	}
}

impl<'c, 'cx, 'o> FusedIterator for ObjectIter<'c, 'cx, 'o> {}
