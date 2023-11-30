/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::future::Future;
use std::mem::transmute;
use std::ops::{Deref, DerefMut};
use std::ptr;

use futures::executor::block_on;
use libffi::high::ClosureOnce3;
use mozjs::glue::JS_GetPromiseResult;
use mozjs::jsapi::{
	AddPromiseReactions, GetPromiseID, GetPromiseState, Heap, IsPromiseObject, JSContext, JSObject, NewPromiseObject,
	PromiseState, RejectPromise, ResolvePromise,
};
use mozjs::jsval::{JSVal, UndefinedValue};
use mozjs::rust::HandleObject;

use crate::{Arguments, Context, Function, Object, Root, Value};
use crate::conversions::ToValue;
use crate::exception::ThrowException;
use crate::flags::PropertyFlags;
use crate::functions::NativeFunction;

/// Represents a [Promise] in the JavaScript Runtime.
/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise) for more details.
#[derive(Debug)]
pub struct Promise {
	promise: Root<Box<Heap<*mut JSObject>>>,
}

impl Promise {
	/// Creates a new [Promise] which never resolves.
	pub fn new(cx: &Context) -> Promise {
		Promise {
			promise: cx.root_object(unsafe { NewPromiseObject(cx.as_ptr(), HandleObject::null().into()) }),
		}
	}

	/// Creates a new [Promise] with an executor.
	/// The executor is a function that takes in two functions, `resolve` and `reject`.
	/// `resolve` and `reject` can be called with a [Value] to resolve or reject the promise with the given value.
	pub fn new_with_executor<F>(cx: &Context, executor: F) -> Option<Promise>
	where
		F: FnOnce(&Context, Function, Function) -> crate::Result<()> + 'static,
	{
		unsafe {
			let native = move |cx: *mut JSContext, argc: u32, vp: *mut JSVal| {
				let cx = Context::new_unchecked(cx);
				let args = Arguments::new(&cx, argc, vp);

				let resolve_obj = args.value(0).unwrap().to_object(&cx).into_root();
				let reject_obj = args.value(1).unwrap().to_object(&cx).into_root();
				let resolve = Function::from_object(&cx, &resolve_obj).unwrap();
				let reject = Function::from_object(&cx, &reject_obj).unwrap();

				match executor(&cx, resolve, reject) {
					Ok(()) => true as u8,
					Err(error) => {
						error.throw(&cx);
						false as u8
					}
				}
			};
			let closure = ClosureOnce3::new(native);
			let fn_ptr: &NativeFunction = transmute(closure.code_ptr());

			let function = Function::new(cx, "executor", Some(*fn_ptr), 2, PropertyFlags::empty());
			let executor = function.to_object(cx);
			let promise = NewPromiseObject(cx.as_ptr(), executor.handle().into());

			if !promise.is_null() {
				Some(Promise { promise: cx.root_object(promise) })
			} else {
				None
			}
		}
	}

	/// Creates a new [Promise] with a [Future].
	/// The future is run to completion on the current thread and cannot interact with an asynchronous runtime.
	///
	/// The [Result] of the future determines if the promise is resolved or rejected.
	pub fn block_on_future<F, Output, Error>(cx: &Context, future: F) -> Option<Promise>
	where
		F: Future<Output = Result<Output, Error>> + 'static,
		Output: ToValue + 'static,
		Error: ToValue + 'static,
	{
		Promise::new_with_executor(cx, move |cx, resolve, reject| {
			let null = Object::null(cx);
			block_on(async move {
				match future.await {
					Ok(output) => match output.to_value(cx) {
						Ok(value) => {
							let _ = resolve.call(cx, &null, &[value]);
						}
						Err(error) => {
							let _ = reject.call(cx, &null, &[error.to_value(cx).unwrap()]);
						}
					},
					Err(error) => {
						let value = match error.to_value(cx) {
							Ok(value) => value,
							Err(error) => error.to_value(cx).unwrap(),
						};
						let _ = reject.call(cx, &null, &[value]);
					}
				}
			});
			Ok(())
		})
	}

	/// Creates a [Promise] from an object.
	pub fn from(object: Root<Box<Heap<*mut JSObject>>>) -> Option<Promise> {
		if Promise::is_promise(&object) {
			Some(Promise { promise: object })
		} else {
			None
		}
	}

	/// Creates a [Promise] from aj object
	///
	/// ### Safety
	/// Object must be a Promise.
	pub unsafe fn from_unchecked(object: Root<Box<Heap<*mut JSObject>>>) -> Promise {
		Promise { promise: object }
	}

	/// Returns the ID of the [Promise].
	pub fn id(&self) -> u64 {
		unsafe { GetPromiseID(self.handle().into()) }
	}

	/// Returns the state of the [Promise].
	///
	/// The state can be `Pending`, `Fulfilled` and `Rejected`.
	pub fn state(&self) -> PromiseState {
		unsafe { GetPromiseState(self.handle().into()) }
	}

	/// Returns the result of the [Promise].
	pub fn result(&self, cx: &Context) -> Value {
		rooted!(in(cx.as_ptr()) let mut result = UndefinedValue());
		unsafe { JS_GetPromiseResult(self.handle().into(), result.handle_mut().into()) }
		Value::from(cx.root(result.get()))
	}

	/// Adds Reactions to the [Promise]
	///
	/// `on_resolved` is similar to calling `.then()` on a promise.
	///
	/// `on_rejected` is similar to calling `.catch()` on a promise.
	pub fn add_reactions(&self, cx: &'_ Context, on_resolved: Option<Function>, on_rejected: Option<Function>) -> bool {
		rooted!(in(cx.as_ptr()) let mut resolved: *mut JSObject = ptr::null_mut());
		rooted!(in(cx.as_ptr()) let mut rejected: *mut JSObject = ptr::null_mut());
		if let Some(on_resolved) = on_resolved {
			resolved.set(on_resolved.to_object(cx).handle().get());
		}
		if let Some(on_rejected) = on_rejected {
			rejected.set(on_rejected.to_object(cx).handle().get());
		}
		unsafe {
			AddPromiseReactions(
				cx.as_ptr(),
				self.handle().into(),
				resolved.handle().into(),
				rejected.handle().into(),
			)
		}
	}

	/// Resolves the [Promise] with the given [Value].
	pub fn resolve(&self, cx: &Context, value: &Value) -> bool {
		unsafe { ResolvePromise(cx.as_ptr(), self.handle().into(), value.handle().into()) }
	}

	/// Rejects the [Promise] with the given [Value].
	pub fn reject(&self, cx: &Context, value: &Value) -> bool {
		unsafe { RejectPromise(cx.as_ptr(), self.handle().into(), value.handle().into()) }
	}

	/// Checks if a [*mut] [JSObject] is a promise.
	pub fn is_promise_raw(cx: &Context, object: *mut JSObject) -> bool {
		rooted!(in(cx.as_ptr()) let object = object);
		unsafe { IsPromiseObject(object.handle().into()) }
	}

	/// Checks if an object is a promise.
	pub fn is_promise(object: &Root<Box<Heap<*mut JSObject>>>) -> bool {
		unsafe { IsPromiseObject(object.handle().into()) }
	}
}

impl Deref for Promise {
	type Target = Root<Box<Heap<*mut JSObject>>>;

	fn deref(&self) -> &Self::Target {
		&self.promise
	}
}

impl DerefMut for Promise {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.promise
	}
}
