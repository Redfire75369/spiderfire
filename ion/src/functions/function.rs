/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::conversions::{ConversionResult, FromJSValConvertible, jsstr_to_string, ToJSValConvertible};
use mozjs::error::throw_type_error;
use mozjs::jsapi::{HandleValueArray, JSFunction, JSFunctionSpec, JSTracer, Value};
use mozjs::jsapi::{
	AssertSameCompartment, JS_CallFunction, JS_DecompileFunction, JS_GetFunctionArity, JS_GetFunctionDisplayId, JS_GetFunctionId,
	JS_GetFunctionLength, JS_GetFunctionObject, JS_GetObjectFunction, JS_IsBuiltinEvalFunction, JS_IsBuiltinFunctionConstructor, JS_IsConstructor,
	JS_IsFunctionBound, JS_NewFunction, JS_ObjectIsFunction, NewFunctionFromSpec1,
};
use mozjs::jsval::{ObjectValue, UndefinedValue};
use mozjs::rust::{CustomTrace, HandleValue, maybe_wrap_object_value, MutableHandleValue};

use crate::exception::{ErrorReport, Exception};
use crate::IonContext;
use crate::objects::object::{IonObject, IonRawObject};

pub type IonNativeFunction = unsafe extern "C" fn(IonContext, u32, *mut Value) -> bool;
pub type IonRawFunction = *mut JSFunction;

#[derive(Clone, Copy, Debug)]
pub struct IonFunction {
	fun: IonRawFunction,
}

impl IonFunction {
	/// Returns the wrapped [IonRawFunction].
	pub unsafe fn raw(&self) -> IonRawFunction {
		self.fun
	}

	/// Creates a new [IonFunction] with the given name, native function, number of arguments and flags.
	pub unsafe fn new(cx: IonContext, name: &str, func: Option<IonNativeFunction>, nargs: u32, flags: u32) -> IonFunction {
		let name = format!("{}\0", name);
		IonFunction::from(JS_NewFunction(cx, func, nargs, flags, name.as_ptr() as *const i8))
	}

	/// Creates a new [IonFunction] with the given function spec.
	pub unsafe fn from_spec(cx: IonContext, spec: *const JSFunctionSpec) -> IonFunction {
		IonFunction::from(NewFunctionFromSpec1(cx, spec))
	}

	/// Creates a new [IonFunction] from an [IonRawFunction].
	pub unsafe fn from(fun: IonRawFunction) -> IonFunction {
		IonFunction { fun }
	}

	/// Creates a new [IonFunction] from an [IonRawObject].
	///
	/// Returns [None] if the object is not a function.
	pub unsafe fn from_object(cx: IonContext, obj: IonRawObject) -> Option<IonFunction> {
		if IonFunction::is_function_raw(obj) {
			Some(IonFunction {
				fun: JS_GetObjectFunction(obj),
			})
		} else {
			throw_type_error(cx, "Object cannot be converted to Function");
			None
		}
	}

	/// Creates a new [IonFunction] from an [IonRawObject].
	///
	/// Returns [None] if the object is not a function.
	pub unsafe fn from_value(cx: IonContext, val: Value) -> Option<IonFunction> {
		if val.is_object() {
			IonFunction::from_object(cx, val.to_object())
		} else {
			None
		}
	}

	/// Converts a [IonFunction] to a [IonRawObject].
	pub unsafe fn to_object(&self) -> IonRawObject {
		JS_GetFunctionObject(self.fun)
	}

	/// Converts a [IonFunction] to a [Value].
	pub unsafe fn to_value(&self) -> Value {
		ObjectValue(self.to_object())
	}

	/// Converts a [IonFunction] to a [String].
	pub unsafe fn to_string(&self, cx: IonContext) -> String {
		rooted!(in(cx) let fun = self.fun);
		let str = JS_DecompileFunction(cx, fun.handle().into());
		jsstr_to_string(cx, str)
	}

	/// Returns the name of a function.
	pub unsafe fn name(&self, cx: IonContext) -> Option<String> {
		let id = JS_GetFunctionId(self.fun);

		if !id.is_null() {
			Some(jsstr_to_string(cx, id))
		} else {
			None
		}
	}

	/// Returns the display name of a function.
	pub unsafe fn display_name(&self, cx: IonContext) -> Option<String> {
		let id = JS_GetFunctionDisplayId(self.fun);
		if !id.is_null() {
			Some(jsstr_to_string(cx, id))
		} else {
			None
		}
	}

	/// Returns the number of arguments of a function.
	pub unsafe fn nargs(&self) -> u16 {
		JS_GetFunctionArity(self.fun)
	}

	/// Returns the length of a function.
	pub unsafe fn length(&self, cx: IonContext) -> Option<u16> {
		rooted!(in(cx) let fun = self.fun);
		let mut length = 0;
		if JS_GetFunctionLength(cx, fun.handle().into(), &mut length) {
			Some(length)
		} else {
			None
		}
	}

	/// Calls a function with the given `this` object and arguments.
	///
	/// Returns [Err] if the function call fails or an exception occurs.
	pub unsafe fn call(&self, cx: IonContext, this: IonObject, args: HandleValueArray) -> Result<Value, Option<ErrorReport>> {
		rooted!(in(cx) let fun = self.fun);
		rooted!(in(cx) let this = this.raw());
		rooted!(in(cx) let mut rval = UndefinedValue());

		if JS_CallFunction(cx, this.handle().into(), fun.handle().into(), &args, rval.handle_mut().into()) {
			Ok(rval.get())
		} else if let Some(exception) = Exception::new(cx) {
			Err(Some(ErrorReport::new(exception)))
		} else {
			Err(None)
		}
	}

	/// Calls a function with the given `this` object and arguments as a [Vec].
	///
	/// Returns [Err] if the function call fails or an exception occurs.
	pub unsafe fn call_with_vec(&self, cx: IonContext, this: IonObject, args: Vec<Value>) -> Result<Value, Option<ErrorReport>> {
		self.call(cx, this, HandleValueArray::from_rooted_slice(args.as_slice()))
	}

	/// Checks if an [IonRawObject] is a function.
	pub unsafe fn is_function_raw(obj: IonRawObject) -> bool {
		JS_ObjectIsFunction(obj)
	}

	/// Checks if a function is bound.
	pub unsafe fn is_bound(&self) -> bool {
		JS_IsFunctionBound(self.fun)
	}

	/// Checks if a function is the built-in eval function.
	pub unsafe fn is_eval(&self) -> bool {
		JS_IsBuiltinEvalFunction(self.fun)
	}

	/// Checks if a function is a constructor.
	pub unsafe fn is_constructor(&self) -> bool {
		JS_IsConstructor(self.fun)
	}

	/// Checks if a function is a built-in function constructor.
	pub unsafe fn is_function_constructor(&self) -> bool {
		JS_IsBuiltinFunctionConstructor(self.fun)
	}
}

impl FromJSValConvertible for IonFunction {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: IonContext, value: HandleValue, _option: ()) -> Result<ConversionResult<IonFunction>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "Value is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		if let Some(fun) = IonFunction::from_object(cx, value.to_object()) {
			Ok(ConversionResult::Success(fun))
		} else {
			Err(())
		}
	}
}

impl ToJSValConvertible for IonFunction {
	#[inline]
	unsafe fn to_jsval(&self, cx: IonContext, mut rval: MutableHandleValue) {
		rval.set(self.to_value());
		maybe_wrap_object_value(cx, rval);
	}
}

unsafe impl CustomTrace for IonFunction {
	fn trace(&self, tracer: *mut JSTracer) {
		unsafe { self.to_object().trace(tracer) }
	}
}
