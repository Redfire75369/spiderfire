/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::future::Future;
use std::mem::transmute;
use std::ops::Deref;

use futures::executor::block_on;
use libffi::high::ClosureOnce3;
use mozjs::jsapi::{
	AddPromiseReactions, GetPromiseID, GetPromiseResult, GetPromiseState, IsPromiseObject, JSContext, JSObject, NewPromiseObject, PromiseState,
	RejectPromise, ResolvePromise,
};
use mozjs::jsval::JSVal;
use mozjs::rust::{Handle, HandleObject, MutableHandle};

use crate::{Arguments, Context, Function, Local, Object, Value};
use crate::conversions::ToValue;
use crate::error::ThrowException;
use crate::flags::PropertyFlags;

#[derive(Debug)]
pub struct Promise<'cx> {
	promise: &'cx mut Local<'cx, *mut JSObject>,
}

impl<'cx> Promise<'cx> {
	/// Creates a new [Promise] which resolves immediately and returns void.
	pub fn new(cx: &'cx Context) -> Promise<'cx> {
		Promise {
			promise: cx.root_object(unsafe { NewPromiseObject(**cx, HandleObject::null().into()) }),
		}
	}

	/// Creates a new [Promise] with an executor.
	///
	/// The executor is a function that takes in two functions, `resolve` and `reject`.
	/// `resolve` and `reject` can be called with a [JSVal] to resolve or reject the promise with the given [JSVal].
	pub fn new_with_executor<F>(cx: &'cx Context, executor: F) -> Option<Promise<'cx>>
	where
		F: for<'cx2> FnOnce(&'cx2 Context, Function<'cx2>, Function<'cx2>) -> crate::Result<()> + 'static,
	{
		unsafe {
			let native = move |mut cx: *mut JSContext, argc: u32, vp: *mut JSVal| {
				let cx = Context::new(&mut cx);
				let args = Arguments::new(&cx, argc, vp);

				let resolve_obj = args.value(0).unwrap().to_object(&cx).into_local();
				let reject_obj = args.value(1).unwrap().to_object(&cx).into_local();
				let resolve = Function::from_object(&cx, resolve_obj).unwrap();
				let reject = Function::from_object(&cx, reject_obj).unwrap();

				match executor(&cx, resolve, reject) {
					Ok(()) => true as u8,
					Err(error) => {
						error.throw(&cx);
						false as u8
					}
				}
			};
			let closure = ClosureOnce3::new(native);
			let fn_ptr = transmute::<_, &unsafe extern "C" fn(*mut JSContext, u32, *mut JSVal) -> bool>(closure.code_ptr());

			let function = Function::new(cx, "executor", Some(*fn_ptr), 2, PropertyFlags::empty());
			let executor = function.to_object(cx);
			let promise = NewPromiseObject(**cx, executor.handle().into());

			if !promise.is_null() {
				Some(Promise { promise: cx.root_object(promise) })
			} else {
				None
			}
		}
	}

	/// Creates a new [Promise] with a [Future].
	///
	/// If the future returns an [Ok], the promise is resolved with the [JSVal] contained within.
	/// If the future returns an [Err], the promise is rejected with the [JSVal] contained within.
	pub fn new_with_future<F, Output, Error>(cx: &'cx Context, future: F) -> Option<Promise<'cx>>
	where
		F: Future<Output = Result<Output, Error>> + 'static,
		Output: for<'cx2> ToValue<'cx2> + 'static,
		Error: for<'cx2> ToValue<'cx2> + 'static,
	{
		let mut future = Some(future);
		Promise::new_with_executor(cx, move |cx, resolve, reject| {
			let null = Object::null(cx);
			block_on(async {
				unsafe {
					let future = future.take().unwrap();
					match future.await {
						Ok(v) => {
							let mut value = Value::undefined(cx);
							v.to_value(cx, &mut value);
							if let Err(Some(error)) = resolve.call(cx, &null, &[value]) {
								println!("{}", error.format(cx));
							}
						}
						Err(v) => {
							let mut value = Value::undefined(cx);
							v.to_value(cx, &mut value);
							if let Err(Some(error)) = reject.call(cx, &null, &[value]) {
								println!("{}", error.format(cx));
							}
						}
					}
				}
			});
			Ok(())
		})
	}

	/// Creates a [Promise] from a [*mut JSObject].
	pub fn from(object: &'cx mut Local<'cx, *mut JSObject>) -> Option<Promise<'cx>> {
		if Promise::is_promise(&object) {
			Some(Promise { promise: object })
		} else {
			None
		}
	}

	/// Creates a [Promise] from a [*mut JSObject].
	///
	/// ### Safety
	/// Object must be a Promise.
	pub unsafe fn from_unchecked(object: &'cx mut Local<'cx, *mut JSObject>) -> Promise<'cx> {
		Promise { promise: object }
	}

	/// Returns the ID of the [Promise].
	pub fn get_id(&self) -> u64 {
		unsafe { GetPromiseID(self.handle().into()) }
	}

	/// Returns the state of the [Promise].
	///
	/// The state can be `Pending`, `Fulfilled` (Resolved) and `Rejected`.
	pub fn get_state(&self) -> PromiseState {
		unsafe { GetPromiseState(self.handle().into()) }
	}

	/// Returns the result of the [Promise].
	pub fn result(&self) -> JSVal {
		unsafe { GetPromiseResult(self.handle().into()) }
	}

	/// Adds Reactions to the [Promise]
	/// `on_resolved` is similar to calling `.then()` on a promise.
	/// `on_rejected` is similar to calling `.catch()` on a promise.
	pub fn add_reactions(&mut self, cx: &'cx Context, on_resolved: Option<Function<'cx>>, on_rejected: Option<Function<'cx>>) -> bool {
		let mut resolved = Object::null(cx);
		let mut rejected = Object::null(cx);
		if let Some(on_resolved) = on_resolved {
			resolved.handle_mut().set(**on_resolved.to_object(cx));
		}
		if let Some(on_rejected) = on_rejected {
			rejected.handle_mut().set(**on_rejected.to_object(cx));
		}
		unsafe { AddPromiseReactions(**cx, self.handle().into(), resolved.handle().into(), rejected.handle().into()) }
	}

	/// Resolves the [Promise] with the given value.
	pub fn resolve(&self, cx: &Context, value: &Value) -> bool {
		unsafe { ResolvePromise(**cx, self.handle().into(), value.handle().into()) }
	}

	/// Rejects the [Promise] with the given [JSVal].
	pub fn reject(&self, cx: &Context, value: &Value) -> bool {
		unsafe { RejectPromise(**cx, self.handle().into(), value.handle().into()) }
	}

	pub fn handle<'a>(&'a self) -> Handle<'a, *mut JSObject>
	where
		'cx: 'a,
	{
		self.promise.handle()
	}

	pub fn handle_mut<'a>(&'a mut self) -> MutableHandle<'a, *mut JSObject>
	where
		'cx: 'a,
	{
		self.promise.handle_mut()
	}

	/// Checks if an object is a promise.
	pub fn is_promise_raw(cx: &Context, object: *mut JSObject) -> bool {
		rooted!(in(**cx) let object = object);
		unsafe { IsPromiseObject(object.handle().into()) }
	}

	/// Checks if an object is a promise.
	pub fn is_promise(object: &Local<*mut JSObject>) -> bool {
		unsafe { IsPromiseObject(object.handle().into()) }
	}
}

impl<'cx> Deref for Promise<'cx> {
	type Target = Local<'cx, *mut JSObject>;

	fn deref(&self) -> &Self::Target {
		&self.promise
	}
}
