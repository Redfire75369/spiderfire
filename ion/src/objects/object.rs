/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::CString;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};

use mozjs::conversions::jsstr_to_string;
use mozjs::glue::{RUST_JSID_IS_INT, RUST_JSID_IS_STRING, RUST_JSID_TO_INT, RUST_JSID_TO_STRING};
use mozjs::jsapi::{
	CurrentGlobalOrNull, GetPropertyKeys, JS_DefineFunction, JS_DefineFunctions, JS_DefineProperty, JS_DeleteProperty1, JS_GetProperty,
	JS_HasOwnProperty, JS_HasProperty, JS_NewPlainObject, JS_SetProperty, JSFunctionSpec, JSObject,
};
use mozjs::jsval::{NullValue, UndefinedValue};
use mozjs::rust::{Handle, IdVector, MutableHandle};

use crate::{Context, Exception, Function, Local, Value};
use crate::conversions::{FromValue, ToValue};
use crate::flags::{IteratorFlags, PropertyFlags};
use crate::functions::NativeFunction;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Key {
	Int(i32),
	String(String),
	Void,
}

impl Display for Key {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match *self {
			Key::Int(int) => f.write_str(&int.to_string()),
			Key::String(ref string) => f.write_str(string),
			Key::Void => panic!("Cannot convert void key into string."),
		}
	}
}

#[derive(Debug)]
pub struct Object<'o> {
	object: Local<'o, *mut JSObject>,
}

impl<'o> Object<'o> {
	/// Creates an empty [Object].
	pub fn new<'cx>(cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(unsafe { JS_NewPlainObject(**cx) }))
	}

	/// Creates a `null` "Object".
	pub fn null<'cx>(cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(NullValue().to_object_or_null()))
	}

	/// Returns the current global object or `null` if one has not been initialised yet.
	pub fn global<'cx>(cx: &'cx Context) -> Object<'cx> {
		Object::from(cx.root_object(unsafe { CurrentGlobalOrNull(**cx) }))
	}

	/// Checks if the [Object] has a value at the given key.
	pub fn has(&self, cx: &Context, key: &str) -> bool {
		let key = CString::new(key).unwrap();
		let mut found = false;

		if unsafe { JS_HasProperty(**cx, self.handle().into(), key.as_ptr() as *const i8, &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Checks if the [Object] has its own value at the given key.
	/// An object owns its properties if they are not inherited from a prototype.
	pub fn has_own(&self, cx: &Context, key: &str) -> bool {
		let key = CString::new(key).unwrap();
		let mut found = false;

		if unsafe { JS_HasOwnProperty(**cx, self.handle().into(), key.as_ptr() as *const i8, &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Gets the [JSVal] at the given key of the [Object].
	/// Returns [None] if there is no value at the given key.
	pub fn get<'cx>(&self, cx: &'cx Context, key: &str) -> Option<Value<'cx>> {
		if self.has(cx, key) {
			let key = CString::new(key).unwrap();
			let mut rval = Value::from(cx.root_value(UndefinedValue()));
			unsafe { JS_GetProperty(**cx, self.handle().into(), key.as_ptr() as *const i8, rval.handle_mut().into()) };
			Some(rval)
		} else {
			None
		}
	}

	/// Gets the value at the given key of the [Object]. as a Rust type.
	/// Returns [None] if the object does not contain the key or conversion to the Rust type fails.
	pub fn get_as<'cx, T: FromValue<'cx>>(&self, cx: &'cx Context, key: &str, strict: bool, config: T::Config) -> Option<T> {
		self.get(cx, key).and_then(|val| unsafe { T::from_value(cx, &val, strict, config).ok() })
	}

	/// Sets the [JSVal] at the given key of the [Object].
	/// Returns `false` if the property cannot be set.
	pub fn set(&mut self, cx: &Context, key: &str, value: &Value) -> bool {
		let key = CString::new(key).unwrap();
		unsafe { JS_SetProperty(**cx, self.handle().into(), key.as_ptr() as *const i8, value.handle().into()) }
	}

	/// Sets the Rust type at the given key of the [Object].
	/// Returns `false` if the property cannot be set.
	pub fn set_as<'cx, T: ToValue<'cx> + ?Sized>(&mut self, cx: &'cx Context, key: &str, value: &T) -> bool {
		self.set(cx, key, unsafe { &value.as_value(cx) })
	}

	/// Defines the [JSVal] at the given key of the [Object] with the given attributes.
	/// Returns `false` if the property cannot be defined.
	pub fn define(&mut self, cx: &Context, key: &str, value: &Value, attrs: PropertyFlags) -> bool {
		let key = CString::new(key).unwrap();
		unsafe {
			JS_DefineProperty(
				**cx,
				self.handle().into(),
				key.as_ptr() as *const i8,
				value.handle().into(),
				attrs.bits() as u32,
			)
		}
	}

	/// Defines the Rust type at the given key of the [Object] with the given attributes.
	/// Returns `false` if the property cannot be defined.
	pub fn define_as<'cx, T: ToValue<'cx> + ?Sized>(&mut self, cx: &'cx Context, key: &str, value: &T, attrs: PropertyFlags) -> bool {
		self.define(cx, key, unsafe { &value.as_value(cx) }, attrs)
	}

	/// Defines a method with the given name, and the given number of arguments and attributes on the [Object].
	/// Parameters are similar to [create_function_spec](crate::spec::create_function_spec).
	pub fn define_method<'cx>(&mut self, cx: &'cx Context, name: &str, method: NativeFunction, nargs: u32, attrs: PropertyFlags) -> Function<'cx> {
		let name = CString::new(name).unwrap();
		cx.root_function(unsafe {
			JS_DefineFunction(
				**cx,
				self.handle().into(),
				name.as_ptr() as *const i8,
				Some(method),
				nargs,
				attrs.bits() as u32,
			)
		})
		.into()
	}

	/// Defines methods on the [Object] using the given [JSFunctionSpec]s.
	/// The final element of the `methods` slice must be `JSFunctionSpec::ZERO`.
	/// They can be created through [function_spec](crate::function_spec).
	pub fn define_methods(&mut self, cx: &Context, methods: &[JSFunctionSpec]) -> bool {
		unsafe { JS_DefineFunctions(**cx, self.handle().into(), methods.as_ptr()) }
	}

	/// Deletes the [JSVal] at the given index.
	/// Returns `false` if the element cannot be deleted.
	pub fn delete(&self, cx: &Context, key: &str) -> bool {
		let key = CString::new(key).unwrap();
		unsafe { JS_DeleteProperty1(**cx, self.handle().into(), key.as_ptr() as *const i8) }
	}

	/// Returns a [Vec] of the keys of the [Object].
	/// Each [Key] can be a [String], integer or void.
	pub fn keys(&self, cx: &Context, flags: Option<IteratorFlags>) -> Vec<Key> {
		let flags = flags.unwrap_or(IteratorFlags::OWN_ONLY);
		let mut ids = unsafe { IdVector::new(**cx) };
		unsafe { GetPropertyKeys(**cx, self.handle().into(), flags.bits(), ids.handle_mut()) };
		ids.iter()
			.map(|id| {
				rooted!(in(**cx) let id = *id);
				unsafe {
					if RUST_JSID_IS_INT(id.handle().into()) {
						Key::Int(RUST_JSID_TO_INT(id.handle().into()))
					} else if RUST_JSID_IS_STRING(id.handle().into()) {
						Key::String(jsstr_to_string(**cx, RUST_JSID_TO_STRING(id.handle().into())))
					} else {
						Key::Void
					}
				}
			})
			.collect()
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
