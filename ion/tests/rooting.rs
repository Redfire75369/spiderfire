use std::ptr;

use mozjs::jsapi::{JS_NewGlobalObject, JSAutoRealm, JSContext, OnNewGlobalHookOption};
use mozjs::jsval::JSVal;
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};

use ion::{Arguments, Context, Function, Object, Value};
use ion::conversions::{ConversionBehavior, FromValue};
use ion::flags::PropertyFlags;

fn main() {
	let engine = JSEngine::init().unwrap();
	let runtime = Runtime::new(engine.handle());
	let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
	let c_options = RealmOptions::default();

	let global = unsafe { JS_NewGlobalObject(runtime.cx(), &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
	let _realm = JSAutoRealm::new(runtime.cx(), global);

	let cx = Context::new(runtime.cx()).unwrap();
	let mut global = Object::from(cx.root_object(global));

	let _native = global.define_method(&cx, "native", native, 1, PropertyFlags::all());
	let native: Function = global.get_as(&cx, "native", true, ()).unwrap();

	let args = vec![Value::null(&cx), Value::bool(&cx, true), Value::string(&cx, "Old String")];
	let result = native.call(&cx, &Object::null(&cx), args.as_slice());
	assert!(result.is_ok());
	let result = unsafe { i32::from_value(&cx, result.as_ref().unwrap(), true, ConversionBehavior::EnforceRange) }.unwrap();
	assert_eq!(3, result);

	let _ = Value::string(&cx, "New String");
}

unsafe extern "C" fn native(cx: *mut JSContext, argc: u32, vp: *mut JSVal) -> bool {
	let cx = Context::new_unchecked(cx);
	let mut args = Arguments::new(&cx, argc, vp);

	let mut correct_args = 0;

	if args.value(0).unwrap().handle().is_null() {
		correct_args += 1;
	}

	let arg1 = args.value(1).unwrap().handle();
	if arg1.is_boolean() && arg1.to_boolean() {
		correct_args += 1;
	}

	let arg2 = args.value(2).unwrap();
	if arg2.handle().is_string() && String::from_value(&cx, arg2, false, ()).unwrap() == *"Old String" {
		correct_args += 1;
	}

	let rval = Value::i32(&cx, correct_args);
	args.rval().handle_mut().set(rval.get());
	true
}
