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
use mozjs::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use mozjs::error::throw_type_error;
use mozjs::jsapi::{
	AddPromiseReactions, AssertSameCompartment, GetPromiseID, GetPromiseResult, GetPromiseState, HandleObject, IsPromiseObject, JSObject, JSTracer,
	NewPromiseObject, PromiseState, RejectPromise, ResolvePromise,
};
use mozjs::jsval::{JSVal, ObjectValue, UndefinedValue};
use mozjs::rust::{CustomTrace, HandleValue, maybe_wrap_object_value, MutableHandleValue};

use crate::{Arguments, Context, Function, Object};
use crate::flags::PropertyFlags;

#[derive(Clone, Copy, Debug)]
pub struct Promise {
	obj: *mut JSObject,
}

impl Promise {
	/// Creates a new [Promise] which resolves immediately and returns void.
	pub fn new(cx: Context) -> Promise {
		unsafe {
			Promise {
				obj: NewPromiseObject(cx, HandleObject::null()),
			}
		}
	}

	/// Creates a new [Promise] with an executor.
	///
	/// The executor is a function that takes in two functions, `resolve` and `reject`.
	/// `resolve` and `reject` can be called with a [JSVal] to resolve or reject the promise with the given [JSVal].
	pub fn new_with_executor<F>(cx: Context, executor: F) -> Option<Promise>
	where
		F: FnOnce(Context, Function, Function) -> crate::Result<()> + 'static,
	{
		unsafe {
			let native = move |cx: Context, argc: u32, vp: *mut JSVal| {
				let args = Arguments::new(argc, vp);
				let resolve = Function::from_value(args.value_or_undefined(0));
				let reject = Function::from_value(args.value_or_undefined(1));
				match (resolve, reject) {
					(Some(resolve), Some(reject)) => match executor(cx, resolve, reject) {
						Ok(()) => true as u8,
						Err(error) => {
							error.throw(cx);
							false as u8
						}
					},
					_ => false as u8,
				}
			};
			let closure = ClosureOnce3::new(native);
			let fn_ptr = transmute::<_, &unsafe extern "C" fn(Context, u32, *mut JSVal) -> bool>(closure.code_ptr());
			let function = Function::new(cx, "executor", Some(*fn_ptr), 2, PropertyFlags::empty());
			rooted!(in(cx) let executor = function.to_object());
			let promise = NewPromiseObject(cx, executor.handle().into());
			if !promise.is_null() {
				Some(Promise { obj: promise })
			} else {
				None
			}
		}
	}

	/// Creates a new [Promise] with a [Future].
	///
	/// If the future returns an [Ok], the promise is resolved with the [JSVal] contained within.
	/// If the future returns an [Err], the promise is rejected with the [JSVal] contained within.
	pub fn new_with_future<F, Output, Error>(cx: Context, future: F) -> Option<Promise>
	where
		F: Future<Output = Result<Output, Error>> + 'static,
		Output: ToJSValConvertible + 'static,
		Error: ToJSValConvertible + 'static,
	{
		let mut future = Some(future);
		let null = Object::from(HandleObject::null().get());
		Promise::new_with_executor(cx, move |cx, resolve, reject| {
			block_on(async {
				unsafe {
					let future = future.take().unwrap();
					match future.await {
						Ok(v) => {
							rooted!(in(cx) let mut value = UndefinedValue());
							v.to_jsval(cx, value.handle_mut());
							if let Err(Some(error)) = resolve.call(cx, null, vec![value.get()]) {
								println!("{}", error);
							}
						}
						Err(v) => {
							rooted!(in(cx) let mut value = UndefinedValue());
							v.to_jsval(cx, value.handle_mut());
							if let Err(Some(error)) = reject.call(cx, null, vec![value.get()]) {
								println!("{}", error);
							}
						}
					}
				}
			});
			Ok(())
		})
	}

	/// Creates a [Promise] from a [*mut JSObject].
	pub fn from(cx: Context, obj: *mut JSObject) -> Option<Promise> {
		if Promise::is_promise_raw(cx, obj) {
			Some(Promise { obj })
		} else {
			None
		}
	}

	/// Creaes a [Promise] from a [JSVal].
	pub fn from_value(cx: Context, val: JSVal) -> Option<Promise> {
		if val.is_object() {
			Promise::from(cx, val.to_object())
		} else {
			None
		}
	}

	/// Converts the [Promise] to a [JSVal].
	pub fn to_value(&self) -> JSVal {
		ObjectValue(self.obj)
	}

	/// Returns the ID of the [Promise].
	pub fn get_id(&self, cx: Context) -> u64 {
		rooted!(in(cx) let robj = self.obj);
		unsafe { GetPromiseID(robj.handle().into()) }
	}

	/// Returns the state of the [Promise].
	///
	/// The state can be `Pending`, `Fulfilled` (Resolved) and `Rejected`.
	pub fn get_state(&self, cx: Context) -> PromiseState {
		rooted!(in(cx) let robj = self.obj);
		unsafe { GetPromiseState(robj.handle().into()) }
	}

	/// Returns the result of the [Promise.
	pub fn result(&self, cx: Context) -> JSVal {
		rooted!(in(cx) let robj = self.obj);
		unsafe { GetPromiseResult(robj.handle().into()) }
	}

	/// Adds Reactions to the [Promise]
	/// `on_resolved` is similar to calling `.then()` on a promise.
	/// `on_rejected` is similar to calling `.catch()` on a promise.
	pub fn add_reactions(&mut self, cx: Context, on_resolved: Option<Function>, on_rejected: Option<Function>) -> bool {
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let mut resolved = *Object::null());
		rooted!(in(cx) let mut rejected = *Object::null());
		if let Some(on_resolved) = on_resolved {
			resolved.set(on_resolved.to_object());
		}
		if let Some(on_rejected) = on_rejected {
			rejected.set(on_rejected.to_object());
		}
		unsafe { AddPromiseReactions(cx, robj.handle().into(), resolved.handle().into(), rejected.handle().into()) }
	}

	/// Resolves the [Promise] with the given [JSVal].
	pub fn resolve(&self, cx: Context, val: JSVal) -> bool {
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let rval = val);
		unsafe { ResolvePromise(cx, robj.handle().into(), rval.handle().into()) }
	}

	/// Rejects the [Promise] with the given [JSVal].
	pub fn reject(&self, cx: Context, val: JSVal) -> bool {
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let rval = val);
		unsafe { RejectPromise(cx, robj.handle().into(), rval.handle().into()) }
	}

	/// Checks if a [*mut JSObject] is a promise.
	pub fn is_promise_raw(cx: Context, obj: *mut JSObject) -> bool {
		rooted!(in(cx) let mut robj = obj);
		unsafe { IsPromiseObject(robj.handle().into()) }
	}
}

impl FromJSValConvertible for Promise {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: Context, value: HandleValue, _: ()) -> Result<ConversionResult<Promise>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "JSVal is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		if let Some(promise) = Promise::from(cx, value.to_object()) {
			Ok(ConversionResult::Success(promise))
		} else {
			Err(())
		}
	}
}

impl ToJSValConvertible for Promise {
	#[inline]
	unsafe fn to_jsval(&self, cx: Context, mut rval: MutableHandleValue) {
		rval.set(self.to_value());
		maybe_wrap_object_value(cx, rval);
	}
}

impl Deref for Promise {
	type Target = *mut JSObject;

	fn deref(&self) -> &Self::Target {
		&self.obj
	}
}

unsafe impl CustomTrace for Promise {
	fn trace(&self, tracer: *mut JSTracer) {
		self.obj.trace(tracer)
	}
}
