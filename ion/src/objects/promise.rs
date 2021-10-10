/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::error::throw_type_error;
use mozjs::jsapi::{HandleObject, PromiseState, Value};
use mozjs::jsapi::{AddPromiseReactions, GetPromiseID, GetPromiseState, IsPromiseObject, NewPromiseObject, RejectPromise, ResolvePromise};
use mozjs::jsval::NullValue;

use crate::functions::function::IonFunction;
use crate::IonContext;
use crate::objects::object::IonRawObject;

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
		rooted!(in(cx) let null = NullValue().to_object_or_null());
		unsafe {
			IonPromise {
				obj: NewPromiseObject(cx, null.handle().into()),
			}
		}
	}

	/// Creates a new function with an executor of the form `Fn(IonFunction, IonFunction)`.
	///
	/// ```ignore
	/// #[js_fn]
	/// unsafe fn executor(cx: IonContext, res: IonFunction, rej: IonFunction) -> IonResult<()> {
	/// 	// Code here
	/// }
	///
	/// IonPromise::new_with_executor(cx, IonFunction::new(cx, "executor", Some(executor), 2, 0));
	/// ```
	pub fn new_with_executor(cx: IonContext, executor: IonFunction) -> IonPromise {
		unsafe {
			rooted!(in(cx) let executor = executor.to_object());
			IonPromise {
				obj: NewPromiseObject(cx, executor.handle().into()),
			}
		}
	}

	pub unsafe fn from(cx: IonContext, obj: IonRawObject) -> Option<IonPromise> {
		if IonPromise::is_promise_raw(cx, obj) {
			Some(IonPromise { obj })
		} else {
			throw_type_error(cx, "Object cannot be converted to Array");
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
		if let Some(on_fulfilled) = on_fulfilled {
			rooted!(in(cx) let on_fulfilled = on_fulfilled.to_object());
			if let Some(on_rejected) = on_rejected {
				rooted!(in(cx) let on_rejected = on_rejected.to_object());
				AddPromiseReactions(cx, robj.handle().into(), on_fulfilled.handle().into(), on_rejected.handle().into())
			} else {
				AddPromiseReactions(cx, robj.handle().into(), on_fulfilled.handle().into(), null)
			}
		} else if let Some(on_rejected) = on_rejected {
			rooted!(in(cx) let on_rejected = on_rejected.to_object());
			AddPromiseReactions(cx, robj.handle().into(), null, on_rejected.handle().into())
		} else {
			AddPromiseReactions(cx, robj.handle().into(), null, null)
		}
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
