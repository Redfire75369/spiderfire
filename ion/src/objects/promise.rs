/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::mem::transmute;
use std::ops::Deref;

use futures::executor::block_on;
use futures::Future;
use libffi::high::arity3::ClosureMut3;
use mozjs::conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible};
use mozjs::error::throw_type_error;
use mozjs::jsapi::{
	AddPromiseReactions, AssertSameCompartment, GetPromiseID, GetPromiseState, HandleObject, IsPromiseObject, JSTracer, NewPromiseObject,
	PromiseState, RejectPromise, ResolvePromise, Value,
};
use mozjs::jsval::{ObjectValue, UndefinedValue};
use mozjs::rust::{CustomTrace, HandleValue, maybe_wrap_object_value, MutableHandleValue};

use crate::{IonContext, IonResult};
use crate::functions::arguments::Arguments;
use crate::functions::function::IonFunction;
use crate::objects::object::{IonObject, IonRawObject};

#[derive(Clone, Copy, Debug)]
pub struct IonPromise {
	obj: IonRawObject,
}

impl IonPromise {
	/// Returns the wrapped [IonRawObject].
	pub fn raw(&self) -> IonRawObject {
		self.obj
	}

	pub fn new(cx: IonContext) -> IonPromise {
		unsafe {
			IonPromise {
				obj: NewPromiseObject(cx, HandleObject::null()),
			}
		}
	}

	/// Creates a new promise with an executor
	pub fn new_with_executor<F>(cx: IonContext, mut executor: F) -> Option<IonPromise>
	where
		F: FnMut(IonContext, IonFunction, IonFunction) -> IonResult<()>,
	{
		unsafe {
			let mut native = |cx: IonContext, argc: u32, vp: *mut Value| {
				let args = Arguments::new(argc, vp);
				let resolve = IonFunction::from_value(cx, args.value_or_undefined(0));
				let reject = IonFunction::from_value(cx, args.value_or_undefined(1));
				if resolve.is_some() && reject.is_some() {
					match executor(cx, resolve.unwrap(), reject.unwrap()) {
						Ok(()) => true as u8,
						Err(error) => {
							error.throw(cx);
							false as u8
						}
					}
				} else {
					false as u8
				}
			};
			let closure = ClosureMut3::new(&mut native);
			let fn_ptr = transmute::<_, &unsafe extern "C" fn(IonContext, u32, *mut Value) -> bool>(closure.code_ptr());
			let function = IonFunction::new(cx, "executor", Some(*fn_ptr), 2, 0);
			rooted!(in(cx) let executor = function.to_object());
			let promise = NewPromiseObject(cx, executor.handle().into());
			if !promise.is_null() {
				Some(IonPromise { obj: promise })
			} else {
				None
			}
		}
	}

	/// Creates a promise with a [Future]
	pub fn new_with_future<F, Output, Error>(cx: IonContext, future: F) -> Option<IonPromise>
	where
		F: Future<Output = Result<Output, Error>>,
		Output: ToJSValConvertible,
		Error: ToJSValConvertible,
	{
		let mut future = Some(future);
		let null = IonObject::from(HandleObject::null().get());
		IonPromise::new_with_executor(cx, |cx, resolve, reject| {
			block_on(async {
				unsafe {
					let future = future.take().unwrap();
					match future.await {
						Ok(v) => {
							rooted!(in(cx) let mut value = UndefinedValue());
							v.to_jsval(cx, value.handle_mut());
							match resolve.call_with_vec(cx, null, vec![value.get()]) {
								Err(Some(error)) => error.print(),
								_ => (),
							}
						}
						Err(v) => {
							rooted!(in(cx) let mut value = UndefinedValue());
							v.to_jsval(cx, value.handle_mut());
							match reject.call_with_vec(cx, null, vec![value.get()]) {
								Err(Some(error)) => error.print(),
								_ => (),
							}
						}
					}
				}
			});
			Ok(())
		})
	}

	pub unsafe fn from(cx: IonContext, obj: IonRawObject) -> Option<IonPromise> {
		if IonPromise::is_promise_raw(cx, obj) {
			Some(IonPromise { obj })
		} else {
			throw_type_error(cx, "Object cannot be converted to Promise");
			None
		}
	}

	pub unsafe fn from_value(cx: IonContext, val: Value) -> Option<IonPromise> {
		if val.is_object() {
			IonPromise::from(cx, val.to_object())
		} else {
			None
		}
	}

	pub fn to_value(&self) -> Value {
		ObjectValue(self.obj)
	}

	pub unsafe fn get_id(&self, cx: IonContext) -> u64 {
		rooted!(in(cx) let robj = self.obj);
		GetPromiseID(robj.handle().into())
	}

	pub unsafe fn get_state(&self, cx: IonContext) -> PromiseState {
		rooted!(in(cx) let robj = self.obj);
		GetPromiseState(robj.handle().into())
	}

	pub unsafe fn add_reactions(&mut self, cx: IonContext, on_fulfilled: Option<IonFunction>, on_rejected: Option<IonFunction>) -> bool {
		let null = HandleObject::null();
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let mut fulfilled = null.get());
		rooted!(in(cx) let mut rejected = null.get());
		if let Some(on_fulfilled) = on_fulfilled {
			fulfilled.set(on_fulfilled.to_object());
		}
		if let Some(on_rejected) = on_rejected {
			rejected.set(on_rejected.to_object());
		}
		AddPromiseReactions(cx, robj.handle().into(), fulfilled.handle().into(), rejected.handle().into())
	}

	pub unsafe fn resolve(&self, cx: IonContext, val: Value) -> bool {
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let rval = val);
		ResolvePromise(cx, robj.handle().into(), rval.handle().into())
	}

	pub unsafe fn reject(&self, cx: IonContext, val: Value) -> bool {
		rooted!(in(cx) let robj = self.obj);
		rooted!(in(cx) let rval = val);
		RejectPromise(cx, robj.handle().into(), rval.handle().into())
	}

	pub unsafe fn is_promise_raw(cx: IonContext, obj: IonRawObject) -> bool {
		rooted!(in(cx) let mut robj = obj);
		IsPromiseObject(robj.handle().into())
	}
}

impl FromJSValConvertible for IonPromise {
	type Config = ();
	#[inline]
	unsafe fn from_jsval(cx: IonContext, value: HandleValue, _option: ()) -> Result<ConversionResult<IonPromise>, ()> {
		if !value.is_object() {
			throw_type_error(cx, "Value is not an object");
			return Err(());
		}

		AssertSameCompartment(cx, value.to_object());
		if let Some(promise) = IonPromise::from(cx, value.to_object()) {
			Ok(ConversionResult::Success(promise))
		} else {
			Err(())
		}
	}
}

impl ToJSValConvertible for IonPromise {
	#[inline]
	unsafe fn to_jsval(&self, cx: IonContext, mut rval: MutableHandleValue) {
		rval.set(self.to_value());
		maybe_wrap_object_value(cx, rval);
	}
}

impl Deref for IonPromise {
	type Target = IonRawObject;

	fn deref(&self) -> &Self::Target {
		&self.obj
	}
}

unsafe impl CustomTrace for IonPromise {
	fn trace(&self, tracer: *mut JSTracer) {
		self.obj.trace(tracer)
	}
}
