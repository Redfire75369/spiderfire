/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::Any;
use std::mem::forget;
use std::thread::Result;

use mozjs::jsval::JSVal;
use mozjs::rust::MutableHandle;

pub use arguments::Arguments;
pub use closure::Closure;
pub use function::{Function, NativeFunction};

use crate::{Context, Error, Object, ResultExc, ThrowException};
use crate::conversions::IntoValue;

mod arguments;
mod closure;
mod function;

pub fn handle_result<T: IntoValue>(cx: &Context, result: ResultExc<T>, mut rval: MutableHandle<JSVal>) -> bool {
	match result {
		Ok(value) => match Box::new(value).into_value(cx) {
			Ok(value) => {
				rval.set(value.get());
				true
			}
			Err(error) => {
				error.throw(cx);
				false
			}
		},
		Err(exception) => {
			exception.throw(cx);
			false
		}
	}
}

#[doc(hidden)]
pub fn handle_native_function_result(cx: &Context, result: Result<ResultExc<bool>>) -> bool {
	match result {
		Ok(Ok(b)) => b,
		Ok(Err(error)) => {
			error.throw(cx);
			false
		}
		Err(unwind_error) => handle_unwind_error(cx, unwind_error),
	}
}

#[doc(hidden)]
pub fn __handle_native_constructor_result(
	cx: &Context, result: Result<ResultExc<()>>, this: &Object, rval: MutableHandle<JSVal>,
) -> bool {
	match result {
		Ok(result) => handle_result(cx, result.map(|_| this), rval),
		Err(unwind_error) => handle_unwind_error(cx, unwind_error),
	}
}

fn handle_unwind_error(cx: &Context, unwind_error: Box<dyn Any + Send>) -> bool {
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
