/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::Any;
use std::mem::forget;
use std::thread::Result;

pub use arguments::Arguments;
pub use closure::Closure;
pub use function::{Function, NativeFunction};

use crate::{ClassDefinition, Context, Error, Object, ResultExc, ThrowException, Value};
use crate::conversions::{IntoValue, ToValue};

mod arguments;
mod closure;
mod function;

#[doc(hidden)]
pub fn __handle_native_function_result<'cx, T: IntoValue<'cx>>(cx: &'cx Context, result: Result<ResultExc<T>>, rval: &mut Value) -> bool {
	match result {
		Ok(Ok(result)) => {
			Box::new(result).into_value(cx, rval);
			true
		}
		Ok(Err(error)) => {
			error.throw(cx);
			false
		}
		Err(unwind_error) => handle_unwind_error(cx, unwind_error),
	}
}

#[doc(hidden)]
pub fn __handle_native_constructor_result<T: ClassDefinition>(cx: &Context, result: Result<ResultExc<T>>, this: &Object, rval: &mut Value) -> bool {
	match result {
		Ok(Ok(result)) => {
			unsafe {
				T::set_private(this.handle().get(), Box::new(result));
			}
			this.to_value(cx, rval);
			true
		}
		Ok(Err(error)) => {
			error.throw(cx);
			false
		}
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
