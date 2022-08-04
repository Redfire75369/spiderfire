/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Display, Formatter};
use std::ops::Deref;

use mozjs::conversions::{ConversionResult, FromJSValConvertible, jsstr_to_string, ToJSValConvertible};
use mozjs::error::throw_type_error;
use mozjs::glue::{RUST_JSID_IS_INT, RUST_JSID_IS_STRING, RUST_JSID_TO_INT, RUST_JSID_TO_STRING};
use mozjs::jsapi::{
	AssertSameCompartment, CurrentGlobalOrNull, GetPropertyKeys, JS_DefineFunction, JS_DefineFunctions, JS_DefineProperty, JS_DeleteProperty1,
	JS_GetProperty, JS_HasOwnProperty, JS_HasProperty, JS_NewPlainObject, JS_SetProperty, JSFunctionSpec, JSObject, JSTracer,
};
use mozjs::jsval::{JSVal, NullValue, ObjectValue, UndefinedValue};
use mozjs::rust::{CustomTrace, HandleValue, IdVector, maybe_wrap_object_value, MutableHandleValue};

use crate::{Context, Exception, Function, NativeFunction};
use crate::flags::{IteratorFlags, PropertyFlags};
use crate::types::values::from_value;

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

#[derive(Clone, Copy, Debug)]
pub struct Object {
	obj: *mut JSObject,
}

impl Object {
	/// Creates an empty [Object].
	pub fn new(cx: Context) -> Object {
		unsafe { Object::from(JS_NewPlainObject(cx)) }
	}

	/// Creates a `null` [Object].
	pub fn null() -> Object {
		Object::from(NullValue().to_object())
	}

	/// Creates an [Object] from a [*mut JSObject].
	pub fn from(obj: *mut JSObject) -> Object {
		Object { obj }
	}

	/// Creates an [Object] from a [JSVal].
	pub fn from_value(val: JSVal) -> Option<Object> {
		if val.is_object() {
			Some(Object::from(val.to_object()))
		} else {
			None
		}
	}

	/// Converts an [Object] to a [JSVal].
	pub fn to_value(&self) -> JSVal {
		ObjectValue(self.obj)
	}

	/// Checks if the [Object] has a value at the given key.
	pub fn has(&self, cx: Context, key: &str) -> bool {
		let key = format!("{}\0", key);
		let mut found = false;
		rooted!(in(cx) let obj = self.obj);

		if unsafe { JS_HasProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Checks if the [Object] has its own value at the given key.
	/// An object owns its properties if they are not inherited from a prototype.
	pub fn has_own(&self, cx: Context, key: &str) -> bool {
		let key = format!("{}\0", key);
		let mut found = false;
		rooted!(in(cx) let obj = self.obj);

		if unsafe { JS_HasOwnProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, &mut found) } {
			found
		} else {
			Exception::clear(cx);
			false
		}
	}

	/// Gets the [JSVal] at the given key of the [Object].
	/// Returns [None] if there is no value at the given key.
	pub fn get(&self, cx: Context, key: &str) -> Option<JSVal> {
		let key = format!("{}\0", key);
		if self.has(cx, &key) {
			rooted!(in(cx) let obj = self.obj);
			rooted!(in(cx) let mut rval = UndefinedValue());
			unsafe { JS_GetProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, rval.handle_mut().into()) };
			Some(rval.get())
		} else {
			None
		}
	}

	/// Gets the value at the given key of the [Object]. as a Rust type.
	/// Returns [None] if the object does not contain the key or conversion to the Rust type fails.
	pub fn get_as<T: FromJSValConvertible>(&self, cx: Context, key: &str, config: T::Config) -> Option<T> {
		let opt = self.get(cx, key);
		opt.and_then(|val| from_value(cx, val, config))
	}

	/// Sets the [JSVal] at the given key of the [Object].
	/// Returns `false` if the property cannot be set.
	pub fn set(&mut self, cx: Context, key: &str, value: JSVal) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let obj = self.obj);
		rooted!(in(cx) let rval = value);
		unsafe { JS_SetProperty(cx, obj.handle().into(), key.as_ptr() as *const i8, rval.handle().into()) }
	}

	/// Sets the Rust type at the given key of the [Object].
	/// Returns `false` if the property cannot be set.
	pub fn set_as<T: ToJSValConvertible>(&mut self, cx: Context, key: &str, value: T) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let mut val = UndefinedValue());
		unsafe {
			value.to_jsval(cx, val.handle_mut());
		}
		self.set(cx, &key, val.get())
	}

	/// Defines the [JSVal] at the given key of the [Object] with the given attributes.
	/// Returns `false` if the property cannot be defined.
	pub fn define(&mut self, cx: Context, key: &str, value: JSVal, attrs: PropertyFlags) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let obj = self.obj);
		rooted!(in(cx) let rval = value);
		unsafe {
			JS_DefineProperty(
				cx,
				obj.handle().into(),
				key.as_ptr() as *const i8,
				rval.handle().into(),
				attrs.bits() as u32,
			)
		}
	}

	/// Defines the Rust type at the given key of the [Object] with the given attributes.
	/// Returns `false` if the property cannot be defined.
	pub fn define_as<T: ToJSValConvertible>(&mut self, cx: Context, key: &str, value: T, attrs: PropertyFlags) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let mut val = UndefinedValue());
		unsafe {
			value.to_jsval(cx, val.handle_mut());
		}
		self.define(cx, &key, val.get(), attrs)
	}

	/// Defines a method with the given name, and the given number of arguments and attributes on the [Object].
	/// Parameters are similar to [create_function_spec](crate::spec::create_function_spec).
	pub fn define_method(&mut self, cx: Context, name: &str, method: NativeFunction, nargs: u32, attrs: PropertyFlags) -> Function {
		let name = format!("{}\0", name);
		rooted!(in(cx) let mut obj = self.obj);
		Function::from(unsafe {
			JS_DefineFunction(
				cx,
				obj.handle().into(),
				name.as_ptr() as *const i8,
				Some(method),
				nargs,
				attrs.bits() as u32,
			)
		})
	}

	/// Defines methods on the [Object] using the given [JSFunctionSpec]s.
	/// The final element of the `methods` slice must be `JSFunctionSpec::ZERO`.
	/// They can be created through [function_spec](crate::spec::function_spec).
	pub fn define_methods(&mut self, cx: Context, methods: &[JSFunctionSpec]) -> bool {
		rooted!(in(cx) let mut obj = self.obj);
		unsafe { JS_DefineFunctions(cx, obj.handle().into(), methods.as_ptr()) }
	}

	/// Deletes the [JSVal] at the given index.
	/// Returns `false` if the element cannot be deleted.
	pub fn delete(&self, cx: Context, key: &str) -> bool {
		let key = format!("{}\0", key);
		rooted!(in(cx) let obj = self.obj);
		unsafe { JS_DeleteProperty1(cx, obj.handle().into(), key.as_ptr() as *const i8) }
	}

	/// Returns a [Vec] of the keys of the [Object].
	/// Each [Key] can be a [String], integer or void.
	pub fn keys(&self, cx: Context, flags: Option<IteratorFlags>) -> Vec<Key> {
		let flags = flags.unwrap_or(IteratorFlags::OWN_ONLY);
		let mut ids = unsafe { IdVector::new(cx) };
		rooted!(in(cx) let obj = self.obj);
		unsafe { GetPropertyKeys(cx, obj.handle().into(), flags.bits(), ids.handle_mut()) };
		ids.iter()
			.map(|id| {
				rooted!(in(cx) let id = *id);
				unsafe {
					if RUST_JSID_IS_INT(id.handle().into()) {
						Key::Int(RUST_JSID_TO_INT(id.handle().into()))
					} else if RUST_JSID_IS_STRING(id.handle().into()) {
						Key::String(jsstr_to_string(cx, RUST_JSID_TO_STRING(id.handle().into())))
					} else {
						Key::Void
					}
				}
			})
			.collect()
	}

	/// Returns the current global [Object] or `null` if one has not been initialised yet.
	pub fn global(cx: Context) -> Object {
		unsafe { Object::from(CurrentGlobalOrNull(cx)) }
	}
}

impl FromJSValConvertible for Object {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: Context, value: HandleValue, _option: ()) -> Result<ConversionResult<Object>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "JSVal is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		Ok(ConversionResult::Success(Object::from(value.to_object())))
	}
}

impl ToJSValConvertible for Object {
	#[inline]
	unsafe fn to_jsval(&self, cx: Context, mut rval: MutableHandleValue) {
		rval.set(ObjectValue(**self));
		maybe_wrap_object_value(cx, rval);
	}
}

impl Deref for Object {
	type Target = *mut JSObject;

	fn deref(&self) -> &Self::Target {
		&self.obj
	}
}

unsafe impl CustomTrace for Object {
	fn trace(&self, tracer: *mut JSTracer) {
		self.obj.trace(tracer)
	}
}
