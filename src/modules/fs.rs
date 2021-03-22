/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::fs::*;
use ::std::path::Path;
use ::std::ptr;

use mozjs::jsapi::*;
use mozjs::jsval::{ObjectValue, UndefinedValue};
use mozjs::typedarray::{CreateWith, Uint8Array};

use crate::runtime::jsapi_utils::string;
use crate::runtime::modules::{register_module, compile_module};

const FS_SOURCE: &'static str = include_str!("js/fs.js");

unsafe extern "C" fn read_binary(cx: *mut JSContext, argc: u32, val: *mut Value) -> bool {
	let args = CallArgs::from_vp(val, argc);

	if args.argc_ > 0 && args.get(0).get().is_string() {
		let str = string::to_string(cx, args.get(0).get());
		let path = Path::new(&str);
		if path.is_file() {
			let file = read(&path);

			if let Ok(bytes) = file {
				rooted!(in(cx) let mut obj = JS_NewPlainObject(cx));
				let array = Uint8Array::create(cx, CreateWith::Slice(bytes.as_slice()), obj.handle_mut());
				if let Ok(_) = array {
					args.rval().set(ObjectValue(obj.get()));
					true
				} else {
					args.rval().set(UndefinedValue());
					false
				}
			} else {
				args.rval().set(UndefinedValue());
				false
			}
		} else {
			args.rval().set(UndefinedValue());
			false
		}
	} else {
		args.rval().set(UndefinedValue());
		false
	}
}

const METHODS: &'static [JSFunctionSpecWithHelp] = &[
	JSFunctionSpecWithHelp {
		name: "readBinary\0".as_ptr() as *const i8,
		call: Some(read_binary),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "readBinary(path)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: ptr::null_mut(),
		call: None,
		nargs: 0,
		flags: 0,
		jitInfo: ptr::null_mut(),
		usage: ptr::null_mut(),
		help: ptr::null_mut(),
	},
];

pub(crate) fn init_fs(cx: *mut JSContext, global: *mut JSObject) -> bool {
	unsafe {
		rooted!(in(cx) let fs_module = JS_NewPlainObject(cx));
		rooted!(in(cx) let undefined = UndefinedValue());
		rooted!(in(cx) let rglobal = global);
		if JS_DefineFunctionsWithHelp(cx, fs_module.handle().into(), METHODS.as_ptr()) {
			rooted!(in(cx) let fs_module_obj = ObjectValue(fs_module.get()));
			if JS_DefineProperty(cx, rglobal.handle().into(), "______fsInternal______\0".as_ptr() as *const i8, fs_module_obj.handle().into(), 0) {
				return register_module(cx, &String::from("fs"), compile_module(cx, &String::from("fs"), &String::from(FS_SOURCE)));
					// && JS_SetProperty(cx, rglobal.handle().into(), "______fsInternal______\0".as_ptr() as *const i8, undefined.handle().into());
			}
		}
		false
	}
}
