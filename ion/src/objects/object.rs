/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::slice;

use mozjs::jsapi::{
	CurrentGlobalOrNull, GetPropertyKeys, JS_DefineFunctionById, JS_DefineFunctions, JS_DefineFunctionsWithHelp, JS_DefinePropertyById2,
	JS_DeletePropertyById, JS_GetPropertyById, JS_HasOwnPropertyById, JS_HasPropertyById, JS_NewPlainObject, JS_SetPropertyById, JSFunctionSpec,
	JSFunctionSpecWithHelp, JSObject,
};
use mozjs::jsapi::PropertyKey as JSPropertyKey;
use mozjs::jsval::NullValue;
use mozjs::rust::IdVector;

use crate::{Context, Exception, Function, Local, OwnedKey, PropertyKey, Value};
use crate::conversions::{FromValue, ToPropertyKey, ToValue};
use crate::flags::{IteratorFlags, PropertyFlags};
use crate::functions::NativeFunction;

/// Represents an [Object] in the JS Runtime.
///
/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object) for more details.
#[derive(Debug)]
pub struct Object<'o> {
	obj: Local<'o, *mut JSObject>,
}

impl<'o> Object<'o> {
	/// Creates a plain empty [Object].
	pub fn new<'cx>(cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(unsafe { JS_NewPlainObject(cx.as_ptr()) }))
	}

	/// Creates a `null` "Object".
	///
	/// Most operations on this will result in an error, so be wary of where it is used.
	pub fn null<'cx>(cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(NullValue().to_object_or_null()))
	}

	/// Returns the current global object or `null` if one has not been initialised yet.
	pub fn global<'cx>(cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(unsafe { CurrentGlobalOrNull(cx.as_ptr()) }))
	}

	/// Checks if the [Object] has a value at the given key.
	pub fn has<'cx, K: ToPropertyKey<'cx>>(&self, cx: &'cx Context, key: K) -> bool {
		let key = key.to_key(cx).unwrap();
		let mut found = false;
		if unsafe { JS_HasPropertyById(cx.as_ptr(), self.handle().into(), key.handle().into(), &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Checks if the [Object] has its own value at the given key.
	///
	/// An object owns its properties if they are not inherited from a prototype.
	pub fn has_own<'cx, K: ToPropertyKey<'cx>>(&self, cx: &'cx Context, key: K) -> bool {
		let key = key.to_key(cx).unwrap();
		let mut found = false;
		if unsafe { JS_HasOwnPropertyById(cx.as_ptr(), self.handle().into(), key.handle().into(), &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Gets the [Value] at the given key of the [Object].
	///
	/// Returns [None] if there is no value at the given key.
	pub fn get<'cx, K: ToPropertyKey<'cx>>(&self, cx: &'cx Context, key: K) -> Option<Value<'cx>> {
		let key = key.to_key(cx).unwrap();
		if self.has(cx, &key) {
			let mut rval = Value::undefined(cx);
			unsafe { JS_GetPropertyById(cx.as_ptr(), self.handle().into(), key.handle().into(), rval.handle_mut().into()) };
			Some(rval)
		} else {
			None
		}
	}

	/// Gets the value at the given key of the [Object]. as a Rust type.
	/// Returns [None] if the object does not contain the key or conversion to the Rust type fails.
	pub fn get_as<'cx, K: ToPropertyKey<'cx>, T: FromValue<'cx>>(&self, cx: &'cx Context, key: K, strict: bool, config: T::Config) -> Option<T> {
		self.get(cx, key).and_then(|val| unsafe { T::from_value(cx, &val, strict, config).ok() })
	}

	/// Sets the [Value] at the given key of the [Object].
	///
	/// Returns `false` if the property cannot be set.
	pub fn set<'cx, K: ToPropertyKey<'cx>>(&mut self, cx: &'cx Context, key: K, value: &Value) -> bool {
		let key = key.to_key(cx).unwrap();
		unsafe { JS_SetPropertyById(cx.as_ptr(), self.handle().into(), key.handle().into(), value.handle().into()) }
	}

	/// Sets the Rust type at the given key of the [Object].
	///
	/// Returns `false` if the property cannot be set.
	pub fn set_as<'cx, K: ToPropertyKey<'cx>, T: ToValue<'cx> + ?Sized>(&mut self, cx: &'cx Context, key: K, value: &T) -> bool {
		self.set(cx, key, unsafe { &value.as_value(cx) })
	}

	/// Defines the [Value] at the given key of the [Object] with the given attributes.
	///
	/// Returns `false` if the property cannot be defined.
	pub fn define<'cx, K: ToPropertyKey<'cx>>(&mut self, cx: &'cx Context, key: K, value: &Value, attrs: PropertyFlags) -> bool {
		let key = key.to_key(cx).unwrap();
		unsafe {
			JS_DefinePropertyById2(
				cx.as_ptr(),
				self.handle().into(),
				key.handle().into(),
				value.handle().into(),
				attrs.bits() as u32,
			)
		}
	}

	/// Defines the Rust type at the given key of the [Object] with the given attributes.
	///
	/// Returns `false` if the property cannot be defined.
	pub fn define_as<'cx, K: ToPropertyKey<'cx>, T: ToValue<'cx> + ?Sized>(
		&mut self, cx: &'cx Context, key: K, value: &T, attrs: PropertyFlags,
	) -> bool {
		self.define(cx, key, unsafe { &value.as_value(cx) }, attrs)
	}

	/// Defines a method with the given name, and the given number of arguments and attributes on the [Object].
	///
	/// Parameters are similar to [create_function_spec](crate::spec::create_function_spec).
	pub fn define_method<'cx, K: ToPropertyKey<'cx>>(
		&mut self, cx: &'cx Context, key: K, method: NativeFunction, nargs: u32, attrs: PropertyFlags,
	) -> Function<'cx> {
		let key = key.to_key(cx).unwrap();
		cx.root_function(unsafe {
			JS_DefineFunctionById(
				cx.as_ptr(),
				self.handle().into(),
				key.handle().into(),
				Some(method),
				nargs,
				attrs.bits() as u32,
			)
		})
		.into()
	}

	/// Defines methods on the [Object] using the given [specs](JSFunctionSpec).
	///
	/// The final element of the `methods` slice must be `JSFunctionSpec::ZERO`.
	#[cfg_attr(feature = "macros", doc = "\nThey can be created through [function_spec](crate::function_spec).")]
	pub fn define_methods(&mut self, cx: &Context, methods: &[JSFunctionSpec]) -> bool {
		unsafe { JS_DefineFunctions(cx.as_ptr(), self.handle().into(), methods.as_ptr()) }
	}

	/// Defines methods on the [Object] using the given [specs with help](JSFunctionSpecWithHelp).
	///
	/// The final element of the `methods` slice must be `JSFunctionSpecWithHelp::ZERO`.
	pub fn define_methods_with_help(&mut self, cx: &Context, methods: &[JSFunctionSpecWithHelp]) -> bool {
		unsafe { JS_DefineFunctionsWithHelp(cx.as_ptr(), self.handle().into(), methods.as_ptr()) }
	}

	/// Deletes the [Value] at the given index.
	///
	/// Returns `false` if the element cannot be deleted.
	pub fn delete<'cx: 'o, K: ToPropertyKey<'cx>>(&self, cx: &'cx Context, key: K) -> bool {
		let key = key.to_key(cx).unwrap();
		let mut result = MaybeUninit::uninit();
		unsafe { JS_DeletePropertyById(cx.as_ptr(), self.handle().into(), key.handle().into(), result.as_mut_ptr()) }
	}

	/// Returns an iterator of the keys of the [Object].
	///
	/// Each key can be a [String], [Symbol](crate::symbol) or integer.
	pub fn keys<'c, 'cx: 'o>(&self, cx: &'cx Context<'c>, flags: Option<IteratorFlags>) -> ObjectKeysIter<'c, 'cx> {
		let flags = flags.unwrap_or(IteratorFlags::OWN_ONLY);
		let mut ids = unsafe { IdVector::new(cx.as_ptr()) };
		unsafe { GetPropertyKeys(cx.as_ptr(), self.handle().into(), flags.bits(), ids.handle_mut()) };
		ObjectKeysIter::new(cx, ids)
	}

	pub fn iter<'c, 'cx, 's>(&'s self, cx: &'cx Context<'c>, flags: Option<IteratorFlags>) -> ObjectIter<'c, 'cx, 'o, 's> {
		ObjectIter::new(cx, self, self.keys(cx, flags))
	}

	pub fn to_hashmap<'cx: 'o>(&self, cx: &'cx Context, flags: Option<IteratorFlags>) -> HashMap<OwnedKey<'cx>, Value<'cx>> {
		self.iter(cx, flags).map(|(k, v)| (k.to_owned_key(cx), v)).collect()
	}

	pub fn into_local(self) -> Local<'o, *mut JSObject> {
		self.obj
	}
}

impl<'o> From<Local<'o, *mut JSObject>> for Object<'o> {
	fn from(obj: Local<'o, *mut JSObject>) -> Object<'o> {
		Object { obj }
	}
}

impl<'o> Deref for Object<'o> {
	type Target = Local<'o, *mut JSObject>;

	fn deref(&self) -> &Self::Target {
		&self.obj
	}
}

impl<'o> DerefMut for Object<'o> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.obj
	}
}

pub struct ObjectKeysIter<'c, 'cx> {
	cx: &'cx Context<'c>,
	keys: IdVector,
	slice: &'static [JSPropertyKey],
	index: usize,
	count: usize,
}

impl<'c, 'cx> ObjectKeysIter<'c, 'cx> {
	fn new(cx: &'cx Context<'c>, keys: IdVector) -> ObjectKeysIter<'c, 'cx> {
		let keys_slice = &*keys;
		let count = keys_slice.len();
		let keys_slice = unsafe { slice::from_raw_parts(keys_slice.as_ptr(), count) };
		ObjectKeysIter {
			cx,
			keys,
			slice: keys_slice,
			index: 0,
			count,
		}
	}
}

impl<'c, 'cx> Drop for ObjectKeysIter<'c, 'cx> {
	fn drop(&mut self) {
		self.slice = &[];
	}
}

impl<'cx> Iterator for ObjectKeysIter<'_, 'cx> {
	type Item = PropertyKey<'cx>;

	fn next(&mut self) -> Option<PropertyKey<'cx>> {
		if self.index < self.count {
			let key = &self.slice[self.index];
			self.index += 1;
			Some(self.cx.root_property_key(*key).into())
		} else {
			None
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.count, Some(self.count))
	}
}

impl<'cx> DoubleEndedIterator for ObjectKeysIter<'_, 'cx> {
	fn next_back(&mut self) -> Option<PropertyKey<'cx>> {
		if self.index < self.count {
			self.count -= 1;
			let key = &self.keys[self.count];
			Some(self.cx.root_property_key(*key).into())
		} else {
			None
		}
	}
}

impl ExactSizeIterator for ObjectKeysIter<'_, '_> {
	fn len(&self) -> usize {
		self.count - self.index
	}
}

impl FusedIterator for ObjectKeysIter<'_, '_> {}

pub struct ObjectIter<'c, 'cx: 'oo, 'oo, 'o> {
	cx: &'cx Context<'c>,
	object: &'o Object<'oo>,
	keys: ObjectKeysIter<'c, 'cx>,
}

impl<'c, 'cx: 'oo, 'oo, 'o> ObjectIter<'c, 'cx, 'oo, 'o> {
	fn new(cx: &'cx Context<'c>, object: &'o Object<'oo>, keys: ObjectKeysIter<'c, 'cx>) -> ObjectIter<'c, 'cx, 'oo, 'o> {
		ObjectIter { cx, object, keys }
	}
}

impl<'cx> Iterator for ObjectIter<'_, 'cx, '_, '_> {
	type Item = (PropertyKey<'cx>, Value<'cx>);

	fn next(&mut self) -> Option<Self::Item> {
		self.keys.next().map(|key| {
			let value = self.object.get(self.cx, &key).unwrap();
			(key, value)
		})
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.keys.size_hint()
	}
}

impl DoubleEndedIterator for ObjectIter<'_, '_, '_, '_> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.keys.next_back().map(|key| {
			let value = self.object.get(self.cx, &key).unwrap();
			(key, value)
		})
	}
}

impl ExactSizeIterator for ObjectIter<'_, '_, '_, '_> {
	fn len(&self) -> usize {
		self.keys.len()
	}
}

impl FusedIterator for ObjectIter<'_, '_, '_, '_> {}
