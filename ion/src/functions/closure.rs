/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use mozjs::glue::JS_GetReservedSlot;
use mozjs::jsapi::{
	GCContext, GetFunctionNativeReserved, JS_NewObject, JS_SetReservedSlot, JSClass, JSCLASS_BACKGROUND_FINALIZE, JSClassOps, JSContext, JSObject,
};
use mozjs::jsval::{JSVal, PrivateValue, UndefinedValue};

use crate::{Arguments, Context, Object, ResultExc, Value};
use crate::conversions::IntoValue;
use crate::functions::__handle_native_function_result;
use crate::objects::class_reserved_slots;

const CLOSURE_SLOT: u32 = 0;

pub type Closure = dyn for<'cx> FnMut(&mut Arguments<'cx>) -> ResultExc<Value<'cx>> + 'static;

pub(crate) fn create_closure_object(cx: &Context, closure: Box<Closure>) -> Object {
	unsafe {
		let object = Object::from(cx.root_object(JS_NewObject(cx.as_ptr(), &CLOSURE_CLASS)));
		JS_SetReservedSlot(
			object.handle().get(),
			CLOSURE_SLOT,
			&PrivateValue(Box::into_raw(Box::new(closure)).cast_const().cast()),
		);
		object
	}
}

pub(crate) unsafe extern "C" fn call_closure(cx: *mut JSContext, argc: u32, vp: *mut JSVal) -> bool {
	let cx = &unsafe { Context::new_unchecked(cx) };
	let args = &mut unsafe { Arguments::new(cx, argc, vp) };

	let callee = cx.root_object(args.call_args().callee());
	let reserved = cx.root_value(unsafe { *GetFunctionNativeReserved(callee.get(), 0) });

	let mut value = UndefinedValue();
	unsafe { JS_GetReservedSlot(reserved.handle().to_object(), CLOSURE_SLOT, &mut value) };
	let closure = unsafe { &mut *(value.to_private() as *mut Box<Closure>) };

	let result = catch_unwind(AssertUnwindSafe(|| {
		closure(args).map(|result| Box::new(result).into_value(cx, args.rval()))
	}));
	__handle_native_function_result(cx, result)
}

unsafe extern "C" fn finalise_closure(_: *mut GCContext, object: *mut JSObject) {
	let mut value = UndefinedValue();
	unsafe {
		JS_GetReservedSlot(object, CLOSURE_SLOT, &mut value);
		let _ = Box::from_raw(value.to_private() as *mut Box<Closure>);
	}
}

static CLOSURE_OPS: JSClassOps = JSClassOps {
	addProperty: None,
	delProperty: None,
	enumerate: None,
	newEnumerate: None,
	resolve: None,
	mayResolve: None,
	finalize: Some(finalise_closure),
	call: None,
	construct: None,
	trace: None,
};

static CLOSURE_CLASS: JSClass = JSClass {
	name: "Closure\0".as_ptr().cast(),
	flags: JSCLASS_BACKGROUND_FINALIZE | class_reserved_slots(1),
	cOps: &CLOSURE_OPS,
	spec: ptr::null_mut(),
	ext: ptr::null_mut(),
	oOps: ptr::null_mut(),
};
