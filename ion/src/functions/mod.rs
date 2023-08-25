/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::c_void;
use std::mem::forget;
use std::thread::Result;

use mozjs::jsapi::JS_SetReservedSlot;
use mozjs::jsval::PrivateValue;

pub use arguments::Arguments;
pub use function::{Function, NativeFunction};

use crate::{Context, Error, Object, ResultExc, ThrowException, Value};
use crate::conversions::{IntoValue, ToValue};

mod arguments;
mod closure;
mod function;

#[doc(hidden)]
pub fn __handle_native_function_result<'cx, T: IntoValue<'cx>>(cx: &'cx Context, result: Result<ResultExc<T>>, rval: &mut Value) -> bool {
	__handle_result(cx, result, move |cx, result| unsafe {
		Box::new(result).into_value(cx, rval);
		true
	})
}

#[doc(hidden)]
pub fn __handle_native_constructor_result<'cx, T: IntoValue<'cx>>(
	cx: &'cx Context, result: Result<ResultExc<T>>, this: &Object<'cx>, rval: &mut Value,
) -> bool {
	__handle_result(cx, result, move |cx, _| unsafe {
		this.to_value(cx, rval);
		true
	})
}

#[doc(hidden)]
pub fn __handle_native_constructor_private_result<'cx, T: IntoValue<'cx>>(
	cx: &'cx Context, result: Result<ResultExc<T>>, index: u32, this: &Object<'cx>, rval: &mut Value,
) -> bool {
	__handle_result(cx, result, move |cx, result| unsafe {
		let b = Box::new(Some(result));
		JS_SetReservedSlot(this.handle().get(), index, &PrivateValue(Box::into_raw(b) as *mut c_void));
		this.to_value(cx, rval);
		true
	})
}

fn __handle_result<'cx, F, T: IntoValue<'cx>>(cx: &'cx Context, result: Result<ResultExc<T>>, callback: F) -> bool
where
	F: FnOnce(&'cx Context, T) -> bool,
{
	match result {
		Ok(Ok(result)) => callback(cx, result),
		Ok(Err(error)) => {
			error.throw(cx);
			false
		}
		Err(unwind_error) => {
			if let Some(unwind) = unwind_error.downcast_ref::<String>() {
				Error::new(unwind, None).throw(cx);
			} else if let Some(unwind) = unwind_error.downcast_ref::<&str>() {
				Error::new(unwind, None).throw(cx);
			} else {
				Error::new("Unknown Panic Occurred", None).throw(cx);
				forget(unwind_error);
			}
			false
		}
	}
}
