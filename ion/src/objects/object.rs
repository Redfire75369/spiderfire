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
	CurrentGlobalOrNull, ESClass, GetBuiltinClass, GetPropertyKeys, Heap, JS_DefineFunctionById, JS_DefineFunctions,
	JS_DefineFunctionsWithHelp, JS_DefineProperties, JS_DefinePropertyById2, JS_DeletePropertyById, JS_GetPropertyById,
	JS_HasOwnPropertyById, JS_HasPropertyById, JS_NewPlainObject, JS_SetPropertyById, JSFunctionSpec,
	JSFunctionSpecWithHelp, JSObject, JSPropertySpec, Unbox,
};
use mozjs::jsapi::PropertyKey as JSPropertyKey;
use mozjs::jsval::{NullValue, UndefinedValue};
use mozjs::rust::IdVector;

use crate::{Context, Error, ErrorKind, Exception, Function, OwnedKey, PropertyKey, Result, Root, Value};
use crate::conversions::{FromValue, ToPropertyKey, ToValue};
use crate::flags::{IteratorFlags, PropertyFlags};
use crate::functions::NativeFunction;

/// Represents an [Object] in the JS Runtime.
///
/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object) for more details.
#[derive(Debug)]
pub struct Object {
	obj: Root<Box<Heap<*mut JSObject>>>,
}

impl Object {
	/// Creates a plain empty [Object].
	pub fn new(cx: &Context) -> Object {
		Object::from(cx.root_object(unsafe { JS_NewPlainObject(cx.as_ptr()) }))
	}

	/// Creates a `null` "Object".
	///
	/// Most operations on this will result in an error, so be wary of where it is used.
	pub fn null(cx: &Context) -> Object {
		Object::from(cx.root_object(NullValue().to_object_or_null()))
	}

	/// Returns the current global object or `null` if one has not been initialised yet.
	pub fn global(cx: &Context) -> Object {
		Object::from(cx.root_object(unsafe { CurrentGlobalOrNull(cx.as_ptr()) }))
	}

	/// Checks if the [Object] has a value at the given key.
	pub fn has<K: ToPropertyKey>(&self, cx: &Context, key: K) -> bool {
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
	pub fn has_own<K: ToPropertyKey>(&self, cx: &Context, key: K) -> bool {
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
	pub fn get<K: ToPropertyKey>(&self, cx: &Context, key: K) -> Option<Value> {
		let key = key.to_key(cx).unwrap();
		if self.has(cx, &key) {
			rooted!(in(cx.as_ptr()) let mut rval = UndefinedValue());
			unsafe {
				JS_GetPropertyById(
					cx.as_ptr(),
					self.handle().into(),
					key.handle().into(),
					rval.handle_mut().into(),
				)
			};
			Some(Value::from(cx.root(rval.get())))
		} else {
			None
		}
	}

	/// Gets the value at the given key of the [Object]. as a Rust type.
	/// Returns [None] if the object does not contain the key or conversion to the Rust type fails.
	pub fn get_as<K: ToPropertyKey, T: FromValue>(
		&self, cx: &Context, key: K, strict: bool, config: T::Config,
	) -> Option<Result<T>> {
		self.get(cx, key).map(|val| T::from_value(cx, &val, strict, config))
	}

	/// Sets the [Value] at the given key of the [Object].
	///
	/// Returns `false` if the property cannot be set.
	pub fn set<K: ToPropertyKey>(&mut self, cx: &Context, key: K, value: &Value) -> bool {
		let key = key.to_key(cx).unwrap();
		unsafe {
			JS_SetPropertyById(
				cx.as_ptr(),
				self.handle().into(),
				key.handle().into(),
				value.handle().into(),
			)
		}
	}

	/// Sets the Rust type at the given key of the [Object].
	///
	/// Returns `false` if the property cannot be set.
	pub fn set_as<K: ToPropertyKey, T: ToValue + ?Sized>(&mut self, cx: &Context, key: K, value: &T) -> Result<()> {
		match value.to_value(cx) {
			Ok(value) => {
				if self.set(cx, key, &value) {
					Ok(())
				} else {
					Err(Error::new("Failed to set key", ErrorKind::Normal))
				}
			}
			Err(error) => Err(error),
		}
	}

	/// Defines the [Value] at the given key of the [Object] with the given attributes.
	///
	/// Returns `false` if the property cannot be defined.
	pub fn define<K: ToPropertyKey>(&mut self, cx: &Context, key: K, value: &Value, attrs: PropertyFlags) -> bool {
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
	pub fn define_as<K: ToPropertyKey, T: ToValue + ?Sized>(
		&mut self, cx: &Context, key: K, value: &T, attrs: PropertyFlags,
	) -> Result<()> {
		match value.to_value(cx) {
			Ok(value) => {
				if self.define(cx, key, &value, attrs) {
					Ok(())
				} else {
					Err(Error::new("Failed to set key", ErrorKind::Normal))
				}
			}
			Err(error) => Err(error),
		}
	}

	/// Defines a method with the given name, and the given number of arguments and attributes on the [Object].
	///
	/// Parameters are similar to [create_function_spec](crate::spec::create_function_spec).
	pub fn define_method<K: ToPropertyKey>(
		&mut self, cx: &Context, key: K, method: NativeFunction, nargs: u32, attrs: PropertyFlags,
	) -> Function {
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

	/// Defines methods on the objects using the given [specs](JSFunctionSpec).
	///
	/// The final element of the `methods` slice must be `JSFunctionSpec::ZERO`.
	#[cfg_attr(
		feature = "macros",
		doc = "\nThey can be created through [function_spec](crate::function_spec)."
	)]
	pub unsafe fn define_methods(&mut self, cx: &Context, methods: &[JSFunctionSpec]) -> bool {
		unsafe { JS_DefineFunctions(cx.as_ptr(), self.handle().into(), methods.as_ptr()) }
	}

	/// Defines methods on the objects using the given [specs](JSFunctionSpecWithHelp), with help.
	///
	/// The final element of the `methods` slice must be `JSFunctionSpecWithHelp::ZERO`.
	pub unsafe fn define_methods_with_help(&mut self, cx: &Context, methods: &[JSFunctionSpecWithHelp]) -> bool {
		unsafe { JS_DefineFunctionsWithHelp(cx.as_ptr(), self.handle().into(), methods.as_ptr()) }
	}

	/// Defines properties on the object using the given [specs](JSPropertySpec).
	///
	/// The final element of the `properties` slice must be `JSPropertySpec::ZERO`.
	pub unsafe fn define_properties(&mut self, cx: &Context, properties: &[JSPropertySpec]) -> bool {
		unsafe { JS_DefineProperties(cx.as_ptr(), self.handle().into(), properties.as_ptr()) }
	}

	/// Deletes the [Value] at the given index.
	///
	/// Returns `false` if the element cannot be deleted.
	pub fn delete<K: ToPropertyKey>(&self, cx: &Context, key: K) -> bool {
		let key = key.to_key(cx).unwrap();
		let mut result = MaybeUninit::uninit();
		unsafe {
			JS_DeletePropertyById(
				cx.as_ptr(),
				self.handle().into(),
				key.handle().into(),
				result.as_mut_ptr(),
			)
		}
	}

	/// Gets the builtin class of the object as described in the ECMAScript specification.
	///
	/// Returns [ESClass::Other] for other projects or proxies that cannot be unwrapped.
	pub fn get_builtin_class(&self, cx: &Context) -> ESClass {
		let mut class = ESClass::Other;
		unsafe {
			GetBuiltinClass(cx.as_ptr(), self.handle().into(), &mut class);
		}
		class
	}

	/// Returns the builtin class of the object if it a wrapper around a primitive.
	///
	/// The boxed types are `Boolean`, `Number`, `String` and `BigInt`
	pub fn is_boxed_primitive(&self, cx: &Context) -> Option<ESClass> {
		let class = self.get_builtin_class(cx);
		match class {
			ESClass::Boolean | ESClass::Number | ESClass::String | ESClass::BigInt => Some(class),
			_ => None,
		}
	}

	/// Unboxes primitive wrappers. See [Object::is_boxed_primitive] for details.
	pub fn unbox_primitive(&self, cx: &Context) -> Option<Value> {
		if self.is_boxed_primitive(cx).is_some() {
			rooted!(in(cx.as_ptr()) let mut rval = UndefinedValue());
			if unsafe { Unbox(cx.as_ptr(), self.handle().into(), rval.handle_mut().into()) } {
				return Some(Value::from(cx.root(rval.get())));
			}
		}
		None
	}

	/// Returns an iterator of the keys of the [Object].
	/// Each key can be a [String], [Symbol](crate::symbol) or integer.
	pub fn keys<'cx>(&self, cx: &'cx Context, flags: Option<IteratorFlags>) -> ObjectKeysIter<'cx> {
		let flags = flags.unwrap_or(IteratorFlags::OWN_ONLY);
		let mut ids = unsafe { IdVector::new(cx.as_ptr()) };
		unsafe { GetPropertyKeys(cx.as_ptr(), self.handle().into(), flags.bits(), ids.handle_mut()) };
		ObjectKeysIter::new(cx, ids)
	}

	pub fn iter<'cx, 's>(&'s self, cx: &'cx Context, flags: Option<IteratorFlags>) -> ObjectIter<'cx, 's> {
		ObjectIter::new(cx, self, self.keys(cx, flags))
	}

	pub fn to_hashmap(&self, cx: &Context, flags: Option<IteratorFlags>) -> HashMap<OwnedKey, Value> {
		self.iter(cx, flags).map(|(k, v)| (k.to_owned_key(cx), v)).collect()
	}

	pub fn into_root(self) -> Root<Box<Heap<*mut JSObject>>> {
		self.obj
	}
}

impl From<Root<Box<Heap<*mut JSObject>>>> for Object {
	fn from(obj: Root<Box<Heap<*mut JSObject>>>) -> Object {
		Object { obj }
	}
}

impl Deref for Object {
	type Target = Root<Box<Heap<*mut JSObject>>>;

	fn deref(&self) -> &Self::Target {
		&self.obj
	}
}

impl DerefMut for Object {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.obj
	}
}

pub struct ObjectKeysIter<'cx> {
	cx: &'cx Context,
	keys: IdVector,
	slice: &'static [JSPropertyKey],
	index: usize,
	count: usize,
}

impl<'cx> ObjectKeysIter<'cx> {
	fn new(cx: &'cx Context, keys: IdVector) -> ObjectKeysIter<'cx> {
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

impl Drop for ObjectKeysIter<'_> {
	fn drop(&mut self) {
		self.slice = &[];
	}
}

impl Iterator for ObjectKeysIter<'_> {
	type Item = PropertyKey;

	fn next(&mut self) -> Option<PropertyKey> {
		if self.index < self.count {
			let key = &self.slice[self.index];
			self.index += 1;
			Some(self.cx.root_property_key(*key).into())
		} else {
			None
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.count - self.index, Some(self.count - self.index))
	}
}

impl DoubleEndedIterator for ObjectKeysIter<'_> {
	fn next_back(&mut self) -> Option<PropertyKey> {
		if self.index < self.count {
			self.count -= 1;
			let key = &self.keys[self.count];
			Some(self.cx.root_property_key(*key).into())
		} else {
			None
		}
	}
}

impl ExactSizeIterator for ObjectKeysIter<'_> {
	fn len(&self) -> usize {
		self.count - self.index
	}
}

impl FusedIterator for ObjectKeysIter<'_> {}

pub struct ObjectIter<'cx, 'o> {
	cx: &'cx Context,
	object: &'o Object,
	keys: ObjectKeysIter<'cx>,
}

impl<'cx, 'o> ObjectIter<'cx, 'o> {
	fn new(cx: &'cx Context, object: &'o Object, keys: ObjectKeysIter<'cx>) -> ObjectIter<'cx, 'o> {
		ObjectIter { cx, object, keys }
	}
}

impl Iterator for ObjectIter<'_, '_> {
	type Item = (PropertyKey, Value);

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

impl DoubleEndedIterator for ObjectIter<'_, '_> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.keys.next_back().map(|key| {
			let value = self.object.get(self.cx, &key).unwrap();
			(key, value)
		})
	}
}

impl ExactSizeIterator for ObjectIter<'_, '_> {
	fn len(&self) -> usize {
		self.keys.len()
	}
}

impl FusedIterator for ObjectIter<'_, '_> {}
