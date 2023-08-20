/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::mem::forget;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::{ptr, thread};

use mozjs::glue::JS_GetReservedSlot;
use mozjs::jsapi::{GCContext, JS_NewObject, JS_SetReservedSlot, JSClass, JSCLASS_BACKGROUND_FINALIZE, JSClassOps, JSContext, JSObject};
use mozjs::jsval::{JSVal, PrivateValue, UndefinedValue};
use mozjs_sys::jsapi::js::GetFunctionNativeReserved;

use crate::{Arguments, Context, Error, Object, Result, ThrowException, Value};
use crate::objects::class_reserved_slots;

const CLOSURE_SLOT: u32 = 0;

pub type Closure = dyn for<'cx> Fn(&'cx Context, &mut Arguments<'cx>) -> Result<Value<'cx>> + 'static;

pub(crate) fn create_closure_object<'cx>(cx: &'cx Context, closure: Box<Closure>) -> Object<'cx> {
	unsafe {
		let object = Object::from(cx.root_object(JS_NewObject(cx.as_ptr(), &CLOSURE_CLASS)));
		JS_SetReservedSlot(
			object.handle().get(),
			CLOSURE_SLOT,
			&PrivateValue(Box::into_raw(Box::new(closure)) as *const _),
		);
		object
	}
}

pub(crate) unsafe extern "C" fn call_closure(cx: *mut JSContext, argc: u32, vp: *mut JSVal) -> bool {
	let cx = &Context::new_unchecked(cx);
	let mut args = Arguments::new(cx, argc, vp);

	let callee = cx.root_object(args.call_args().callee());
	let reserved = cx.root_value(*unsafe { GetFunctionNativeReserved(callee.get(), 0) });

	let mut value = UndefinedValue();
	JS_GetReservedSlot(reserved.handle().to_object(), CLOSURE_SLOT, &mut value);
	let closure = &*(value.to_private() as *mut Box<Closure>);

	let result: thread::Result<Result<Value>> = catch_unwind(AssertUnwindSafe(|| closure(cx, &mut args)));

	match result {
		Ok(Ok(val)) => {
			args.rval().handle_mut().set(val.get());
			true
		}
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

unsafe extern "C" fn finalize_closure(_: *mut GCContext, object: *mut JSObject) {
	let mut value = UndefinedValue();
	JS_GetReservedSlot(object, CLOSURE_SLOT, &mut value);
	let _ = Box::from_raw(value.to_private() as *mut Box<Closure>);
}

static CLOSURE_OPS: JSClassOps = JSClassOps {
	addProperty: None,
	delProperty: None,
	enumerate: None,
	newEnumerate: None,
	resolve: None,
	mayResolve: None,
	finalize: Some(finalize_closure),
	call: None,
	construct: None,
	trace: None,
};

static CLOSURE_CLASS: JSClass = JSClass {
	name: "Closure\0".as_ptr() as *const _,
	flags: JSCLASS_BACKGROUND_FINALIZE | class_reserved_slots(1),
	cOps: &CLOSURE_OPS,
	spec: ptr::null_mut(),
	ext: ptr::null_mut(),
	oOps: ptr::null_mut(),
};
