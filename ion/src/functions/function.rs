/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::conversions::{ConversionResult, FromJSValConvertible, jsstr_to_string, ToJSValConvertible};
use mozjs::error::throw_type_error;
use mozjs::jsapi::{
	AssertSameCompartment, HandleValueArray, JS_CallFunction, JS_DecompileFunction, JS_GetFunctionArity, JS_GetFunctionDisplayId, JS_GetFunctionId,
	JS_GetFunctionLength, JS_GetFunctionObject, JS_GetObjectFunction, JS_IsBuiltinEvalFunction, JS_IsBuiltinFunctionConstructor, JS_IsConstructor,
	JS_IsFunctionBound, JS_NewFunction, JS_ObjectIsFunction, JSFunction, JSFunctionSpec, JSObject, JSTracer, NewFunctionFromSpec1,
};
use mozjs::jsval::{JSVal, ObjectValue, UndefinedValue};
use mozjs::rust::{CustomTrace, HandleValue, maybe_wrap_object_value, MutableHandleValue};

use crate::{Context, ErrorReport, Object};
use crate::flags::PropertyFlags;

pub type NativeFunction = unsafe extern "C" fn(Context, u32, *mut JSVal) -> bool;

#[derive(Clone, Copy, Debug)]
pub struct Function {
	fun: *mut JSFunction,
}

impl Function {
	/// Creates a new [Function] with the given name, native function, number of arguments and flags.
	pub fn new(cx: Context, name: &str, func: Option<NativeFunction>, nargs: u32, flags: PropertyFlags) -> Function {
		let name = format!("{}\0", name);
		unsafe { Function::from(JS_NewFunction(cx, func, nargs, flags.bits() as u32, name.as_ptr() as *const i8)) }
	}

	/// Creates a new [Function] with the given [JSFunctionSpec]
	pub fn from_spec(cx: Context, spec: *const JSFunctionSpec) -> Function {
		Function::from(unsafe { NewFunctionFromSpec1(cx, spec) })
	}

	pub fn from(fun: *mut JSFunction) -> Function {
		Function { fun }
	}

	/// Creates a new [Function] from a [*mut JSObject].
	/// Returns [None] if the object is not a function.
	pub fn from_object(obj: *mut JSObject) -> Option<Function> {
		if Function::is_function_raw(obj) {
			Some(Function {
				fun: unsafe { JS_GetObjectFunction(obj) },
			})
		} else {
			None
		}
	}

	/// Creates a new [Function] from a [*mut JSObject].
	/// Returns [None] if the object is not a function.
	pub fn from_value(val: JSVal) -> Option<Function> {
		if val.is_object() {
			Function::from_object(val.to_object())
		} else {
			None
		}
	}

	/// Converts the [Function] to a [*mut JSObject].
	pub fn to_object(&self) -> *mut JSObject {
		unsafe { JS_GetFunctionObject(self.fun) }
	}

	/// Converts the [Function] to a [JSVal].
	pub fn to_value(&self) -> JSVal {
		ObjectValue(self.to_object())
	}

	/// Converts the [Function] to a [String].
	pub fn to_string(&self, cx: Context) -> String {
		rooted!(in(cx) let fun = self.fun);
		unsafe {
			let str = JS_DecompileFunction(cx, fun.handle().into());
			jsstr_to_string(cx, str)
		}
	}

	/// Returns the name of the function.
	pub fn name(&self, cx: Context) -> Option<String> {
		let id = unsafe { JS_GetFunctionId(self.fun) };

		if !id.is_null() {
			Some(unsafe { jsstr_to_string(cx, id) })
		} else {
			None
		}
	}

	/// Returns the display name of the function.
	pub fn display_name(&self, cx: Context) -> Option<String> {
		let id = unsafe { JS_GetFunctionDisplayId(self.fun) };
		if !id.is_null() {
			Some(unsafe { jsstr_to_string(cx, id) })
		} else {
			None
		}
	}

	/// Returns the number of arguments of the function.
	pub fn nargs(&self) -> u16 {
		unsafe { JS_GetFunctionArity(self.fun) }
	}

	/// Returns the length of the function.
	pub fn length(&self, cx: Context) -> Option<u16> {
		rooted!(in(cx) let fun = self.fun);
		let mut length = 0;
		if unsafe { JS_GetFunctionLength(cx, fun.handle().into(), &mut length) } {
			Some(length)
		} else {
			None
		}
	}

	/// Calls a function with the given `this` [Object] and arguments as a [Vec].
	/// Returns [Err] if the function call fails or an exception occurs.
	pub fn call(&self, cx: Context, this: Object, args: Vec<JSVal>) -> Result<JSVal, Option<ErrorReport>> {
		self.call_with_handle(cx, this, unsafe { HandleValueArray::from_rooted_slice(args.as_slice()) })
	}

	/// Calls a function with the given `this` [Object] and arguments as a [HandleValueArray].
	/// Returns [Err] if the function call fails or an exception occurs.
	pub fn call_with_handle(&self, cx: Context, this: Object, args: HandleValueArray) -> Result<JSVal, Option<ErrorReport>> {
		rooted!(in(cx) let fun = self.fun);
		rooted!(in(cx) let this = *this);
		rooted!(in(cx) let mut rval = UndefinedValue());

		if unsafe { JS_CallFunction(cx, this.handle().into(), fun.handle().into(), &args, rval.handle_mut().into()) } {
			Ok(rval.get())
		} else {
			Err(ErrorReport::new_with_exception_stack(cx))
		}
	}

	/// Checks if an [*mut JSObject] is a function.
	pub fn is_function_raw(obj: *mut JSObject) -> bool {
		unsafe { JS_ObjectIsFunction(obj) }
	}

	/// Checks if a function is bound.
	pub fn is_bound(&self) -> bool {
		unsafe { JS_IsFunctionBound(self.fun) }
	}

	/// Checks if a function is the built-in eval function.
	pub fn is_eval(&self) -> bool {
		unsafe { JS_IsBuiltinEvalFunction(self.fun) }
	}

	/// Checks if a function is a constructor.
	pub fn is_constructor(&self) -> bool {
		unsafe { JS_IsConstructor(self.fun) }
	}

	/// Checks if a function is the built-in function constructor.
	pub fn is_function_constructor(&self) -> bool {
		unsafe { JS_IsBuiltinFunctionConstructor(self.fun) }
	}
}

impl FromJSValConvertible for Function {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: Context, value: HandleValue, _: ()) -> Result<ConversionResult<Function>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "JSVal is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		if let Some(fun) = Function::from_object(value.to_object()) {
			Ok(ConversionResult::Success(fun))
		} else {
			Err(())
		}
	}
}

impl ToJSValConvertible for Function {
	#[inline]
	unsafe fn to_jsval(&self, cx: Context, mut rval: MutableHandleValue) {
		rval.set(self.to_value());
		maybe_wrap_object_value(cx, rval);
	}
}

unsafe impl CustomTrace for Function {
	fn trace(&self, tracer: *mut JSTracer) {
		self.to_object().trace(tracer)
	}
}
