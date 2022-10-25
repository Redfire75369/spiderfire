/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::CString;
use std::ops::Deref;

use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{
	HandleValueArray, JS_CallFunction, JS_DecompileFunction, JS_GetFunctionArity, JS_GetFunctionDisplayId, JS_GetFunctionId, JS_GetFunctionLength,
	JS_GetFunctionObject, JS_GetObjectFunction, JS_IsBuiltinEvalFunction, JS_IsBuiltinFunctionConstructor, JS_IsConstructor, JS_IsFunctionBound,
	JS_NewFunction, JS_ObjectIsFunction, JSContext, JSFunction, JSFunctionSpec, JSObject, NewFunctionFromSpec1,
};
use mozjs::jsval::JSVal;
use mozjs::rust::{Handle, MutableHandle};

use crate::{Context, ErrorReport, Local, Object, Value};
use crate::flags::PropertyFlags;

pub type NativeFunction = unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool;

#[derive(Debug)]
pub struct Function<'cx> {
	function: &'cx mut Local<'cx, *mut JSFunction>,
}

impl<'cx> Function<'cx> {
	/// Creates a new [Function] with the given name, native function, number of arguments and flags.
	pub fn new(cx: &'cx Context, name: &str, func: Option<NativeFunction>, nargs: u32, flags: PropertyFlags) -> Function<'cx> {
		let name = CString::new(name).unwrap();
		Function {
			function: cx.root_function(unsafe { JS_NewFunction(**cx, func, nargs, flags.bits() as u32, name.as_ptr() as *const i8) }),
		}
	}

	/// Creates a new [Function] with the given [JSFunctionSpec]
	pub fn from_spec(cx: &'cx Context, spec: *const JSFunctionSpec) -> Function<'cx> {
		Function {
			function: cx.root_function(unsafe { NewFunctionFromSpec1(**cx, spec) }),
		}
	}

	pub fn from(function: &'cx mut Local<'cx, *mut JSFunction>) -> Function<'cx> {
		Function { function }
	}

	/// Creates a new [Function] from a [*mut JSObject].
	/// Returns [None] if the object is not a function.
	pub fn from_object(cx: &'cx Context, obj: &Local<'cx, *mut JSObject>) -> Option<Function<'cx>> {
		if Function::is_function_raw(**obj) {
			Some(Function {
				function: cx.root_function(unsafe { JS_GetObjectFunction(**obj) }),
			})
		} else {
			None
		}
	}

	/// Converts the [Function] to a [*mut JSObject].
	pub fn to_object(&self, cx: &'cx Context) -> Object<'cx> {
		cx.root_object(unsafe { JS_GetFunctionObject(self.handle().get()) }).into()
	}

	/// Converts the [Function] to a [JSVal].
	pub fn to_value(&self, cx: &'cx Context) -> Value<'cx> {
		Value::object(cx, &self.to_object(cx))
	}

	/// Converts the [Function] to a [String].
	pub fn to_string(&self, cx: &Context) -> String {
		unsafe {
			let str = JS_DecompileFunction(**cx, self.handle().into());
			jsstr_to_string(**cx, str)
		}
	}

	/// Returns the name of the function.
	pub fn name(&self, cx: &Context) -> Option<String> {
		let id = unsafe { JS_GetFunctionId(self.handle().get()) };

		if !id.is_null() {
			Some(unsafe { jsstr_to_string(**cx, id) })
		} else {
			None
		}
	}

	/// Returns the display name of the function.
	pub fn display_name(&self, cx: &Context) -> Option<String> {
		let id = unsafe { JS_GetFunctionDisplayId(self.handle().get()) };
		if !id.is_null() {
			Some(unsafe { jsstr_to_string(**cx, id) })
		} else {
			None
		}
	}

	/// Returns the number of arguments of the function.
	pub fn nargs(&self) -> u16 {
		unsafe { JS_GetFunctionArity(self.handle().get()) }
	}

	/// Returns the length of the function.
	pub fn length(&self, cx: &Context) -> Option<u16> {
		let mut length = 0;
		if unsafe { JS_GetFunctionLength(**cx, self.handle().into(), &mut length) } {
			Some(length)
		} else {
			None
		}
	}

	/// Calls a function with the given `this` [Object] and arguments.
	/// Returns [Err] if the function call fails or an exception occurs.
	pub fn call(&self, cx: &'cx Context, this: &Object, args: &[Value<'cx>]) -> Result<Value<'cx>, Option<ErrorReport>> {
		let args: Vec<_> = args.iter().map(|a| ***a).collect();
		self.call_with_handle(cx, this, unsafe { HandleValueArray::from_rooted_slice(args.as_slice()) })
	}

	/// Calls a function with the given `this` [Object] and arguments as a [HandleValueArray].
	/// Returns [Err] if the function call fails or an exception occurs.
	pub fn call_with_handle(&self, cx: &'cx Context, this: &Object, args: HandleValueArray) -> Result<Value<'cx>, Option<ErrorReport>> {
		let mut rval = Value::undefined(cx);
		if unsafe { JS_CallFunction(**cx, this.handle().into(), self.handle().into(), &args, rval.handle_mut().into()) } {
			Ok(rval)
		} else {
			Err(ErrorReport::new_with_exception_stack(cx))
		}
	}

	/// Checks if a function is bound.
	pub fn is_bound(&self) -> bool {
		unsafe { JS_IsFunctionBound(self.handle().get()) }
	}

	/// Checks if a function is the built-in eval function.
	pub fn is_eval(&self) -> bool {
		unsafe { JS_IsBuiltinEvalFunction(self.handle().get()) }
	}

	/// Checks if a function is a constructor.
	pub fn is_constructor(&self) -> bool {
		unsafe { JS_IsConstructor(self.handle().get()) }
	}

	/// Checks if a function is the built-in function constructor.
	pub fn is_function_constructor(&self) -> bool {
		unsafe { JS_IsBuiltinFunctionConstructor(self.handle().get()) }
	}

	pub fn handle<'a>(&'a self) -> Handle<'a, *mut JSFunction>
	where
		'cx: 'a,
	{
		self.function.handle()
	}

	pub fn handle_mut<'a>(&'a mut self) -> MutableHandle<'a, *mut JSFunction>
	where
		'cx: 'a,
	{
		self.function.handle_mut()
	}

	/// Checks if an [*mut JSObject] is a function.
	pub fn is_function_raw(obj: *mut JSObject) -> bool {
		unsafe { JS_ObjectIsFunction(obj) }
	}
}

impl<'cx> From<&'cx mut Local<'cx, *mut JSFunction>> for Function<'cx> {
	fn from(function: &'cx mut Local<'cx, *mut JSFunction>) -> Function<'cx> {
		Function { function }
	}
}

impl<'cx> Deref for Function<'cx> {
	type Target = Local<'cx, *mut JSFunction>;

	fn deref(&self) -> &Self::Target {
		&*self.function
	}
}
