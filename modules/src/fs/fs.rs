/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::fs;
use ::std::os;
use ::std::path::Path;
use ::std::ptr;

use mozjs::conversions::ToJSValConvertible;
use mozjs::jsapi::*;
use mozjs::jsval::{BooleanValue, ObjectValue, UndefinedValue};
use mozjs::typedarray::{CreateWith, Uint8Array};

use ion::functions::arguments::Arguments;
use ion::types::string::to_string;
use runtime::modules::{compile_module, register_module};

const FS_SOURCE: &str = include_str!("fs.js");

unsafe extern "C" fn read_binary(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 0 {
		let path_str = to_string(cx, args.value(0).unwrap());
		let path = Path::new(&path_str);

		if path.is_file() {
			if let Ok(bytes) = fs::read(&path) {
				rooted!(in(cx) let mut obj = JS_NewPlainObject(cx));
				let array = Uint8Array::create(cx, CreateWith::Slice(bytes.as_slice()), obj.handle_mut());
				if array.is_ok() {
					args.rval().set(ObjectValue(obj.get()));
					return true;
				}
			}
		}
	}

	false
}

unsafe extern "C" fn read_string(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 0 {
		let path_str = to_string(cx, args.value(0).unwrap());
		let path = Path::new(&path_str);

		if path.is_file() {
			if let Ok(str) = fs::read_to_string(&path) {
				rooted!(in(cx) let mut rval = UndefinedValue());
				str.to_jsval(cx, rval.handle_mut());

				args.rval().set(rval.get());
				return true;
			}
		}
	}

	false
}

unsafe extern "C" fn read_dir(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 0 {
		let path_str = to_string(cx, args.value(0).unwrap());
		let path = Path::new(&path_str);

		if path.is_dir() {
			if let Ok(dir) = fs::read_dir(path) {
				let entries: Vec<_> = dir.filter_map(|entry| entry.ok()).collect();
				let mut entry_strings: Vec<_> = entries.iter().map(|entry| entry.file_name().into_string().unwrap()).collect();
				entry_strings.sort();
				let dir_entries: Vec<_> = entry_strings
					.iter()
					.map(|entry_string| {
						rooted!(in(cx) let mut rval = UndefinedValue());
						entry_string.to_jsval(cx, rval.handle_mut());
						rval.get()
					})
					.collect();
				let array = NewArrayObject(cx, &HandleValueArray::from_rooted_slice(dir_entries.as_slice()));

				args.rval().set(ObjectValue(array));
				return true;
			}
		}
	}
	false
}

unsafe extern "C" fn write(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 0 {
		let path_str = to_string(cx, args.value(0).unwrap());
		let path = Path::new(&path_str);

		let contents = to_string(cx, args.value(1).unwrap());

		if !path.is_dir() {
			args.rval().set(BooleanValue(fs::write(path, contents).is_ok()));
			return true;
		}
	}
	false
}

unsafe extern "C" fn create_dir(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 0 {
		let path_str = to_string(cx, args.value(0).unwrap());
		let path = Path::new(&path_str);

		if !path.is_file() {
			args.rval().set(BooleanValue(fs::create_dir(path).is_ok()));
			return true;
		}
	}

	false
}

unsafe extern "C" fn create_dir_recursive(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 0 {
		let path_str = to_string(cx, args.value(0).unwrap());
		let path = Path::new(&path_str);

		if !path.is_file() {
			args.rval().set(BooleanValue(fs::create_dir_all(path).is_ok()));
			return true;
		}
	}

	false
}

unsafe extern "C" fn remove_file(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 0 {
		let path_str = to_string(cx, args.value(0).unwrap());
		let path = Path::new(&path_str);

		if path.is_file() {
			args.rval().set(BooleanValue(fs::remove_file(path).is_ok()));
			return true;
		}
	}

	false
}

unsafe extern "C" fn remove_dir(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 0 {
		let path_str = to_string(cx, args.value(0).unwrap());
		let path = Path::new(&path_str);

		if path.is_dir() {
			args.rval().set(BooleanValue(fs::remove_dir(path).is_ok()));
			return true;
		}
	}

	false
}

unsafe extern "C" fn remove_dir_recursive(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 0 {
		let path_str = to_string(cx, args.value(0).unwrap());
		let path = Path::new(&path_str);

		if path.is_dir() {
			args.rval().set(BooleanValue(fs::remove_dir_all(path).is_ok()));
			return true;
		}
	}

	false
}

unsafe extern "C" fn copy(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 1 {
		let from_str = to_string(cx, args.value(0).unwrap());
		let from = Path::new(&from_str);

		let to_str = to_string(cx, args.value(1).unwrap());
		let to = Path::new(&to_str);

		if !from.is_dir() && !to.is_dir() {
			args.rval().set(BooleanValue(fs::copy(from, to).is_ok()));
			return true;
		}
	}

	false
}

unsafe extern "C" fn rename(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 1 {
		let from_str = to_string(cx, args.value(0).unwrap());
		let from = Path::new(&from_str);

		let to_str = to_string(cx, args.value(1).unwrap());
		let to = Path::new(&to_str);

		args.rval().set(BooleanValue(fs::rename(from, to).is_ok()));
		return true;
	}

	false
}

unsafe extern "C" fn soft_link(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 1 {
		let original_str = to_string(cx, args.value(0).unwrap());
		let original = Path::new(&original_str);

		let link_str = to_string(cx, args.value(1).unwrap());
		let link = Path::new(&link_str);

		if !link.exists() {
			#[cfg(target_family = "unix")]
			{
				args.rval().set(BooleanValue(os::unix::fs::symlink(original, link).is_ok()));
			}
			#[cfg(target_family = "windows")]
			{
				if original.is_file() {
					args.rval().set(BooleanValue(os::windows::fs::symlink_file(original, link).is_ok()));
				} else if original.is_dir() {
					args.rval().set(BooleanValue(os::windows::fs::symlink_dir(original, link).is_ok()));
				} else {
					args.rval().set(BooleanValue(false));
				}
			}

			return true;
		}
	}

	false
}

unsafe extern "C" fn hard_link(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
	let args = Arguments::new(argc, vp);
	args.rval().set(UndefinedValue());

	if args.len() > 1 {
		let original_str = to_string(cx, args.value(0).unwrap());
		let original = Path::new(&original_str);

		let link_str = to_string(cx, args.value(1).unwrap());
		let link = Path::new(&link_str);

		args.rval().set(BooleanValue(fs::hard_link(original, link).is_ok()));
		return true;
	}

	false
}

const METHODS: &[JSFunctionSpecWithHelp] = &[
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
		name: "readString\0".as_ptr() as *const i8,
		call: Some(read_string),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "readString(path)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "readDir\0".as_ptr() as *const i8,
		call: Some(read_dir),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "readDir(path)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "write\0".as_ptr() as *const i8,
		call: Some(write),
		nargs: 2,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "write(path, contents)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "createDir\0".as_ptr() as *const i8,
		call: Some(create_dir),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "createDir(path)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "createDirRecursive\0".as_ptr() as *const i8,
		call: Some(create_dir_recursive),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "createDirRecursive(path)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "removeFile\0".as_ptr() as *const i8,
		call: Some(remove_file),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "removeFile(path)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "removeDir\0".as_ptr() as *const i8,
		call: Some(remove_dir),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "createDir(path)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "removeDirRecursive\0".as_ptr() as *const i8,
		call: Some(remove_dir_recursive),
		nargs: 1,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "removeDirRecursive(path)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "copy\0".as_ptr() as *const i8,
		call: Some(copy),
		nargs: 2,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "copy(from, to)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "rename\0".as_ptr() as *const i8,
		call: Some(rename),
		nargs: 2,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "rename(from, to)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "softLink\0".as_ptr() as *const i8,
		call: Some(soft_link),
		nargs: 2,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "softLink(original, link)\0".as_ptr() as *const i8,
		help: "\0".as_ptr() as *const i8,
	},
	JSFunctionSpecWithHelp {
		name: "hardLink\0".as_ptr() as *const i8,
		call: Some(hard_link),
		nargs: 2,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		jitInfo: ptr::null_mut(),
		usage: "hardLink(original, link)\0".as_ptr() as *const i8,
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

pub fn init_fs(cx: *mut JSContext, global: *mut JSObject) -> bool {
	unsafe {
		rooted!(in(cx) let fs_module = JS_NewPlainObject(cx));
		rooted!(in(cx) let undefined = UndefinedValue());
		rooted!(in(cx) let rglobal = global);
		if JS_DefineFunctionsWithHelp(cx, fs_module.handle().into(), METHODS.as_ptr()) {
			rooted!(in(cx) let fs_module_obj = ObjectValue(fs_module.get()));
			if JS_DefineProperty(
				cx,
				rglobal.handle().into(),
				"______fsInternal______\0".as_ptr() as *const i8,
				fs_module_obj.handle().into(),
				0,
			) {
				return register_module(cx, &String::from("fs"), compile_module(cx, &String::from("fs"), &String::from(FS_SOURCE)));
				// && JS_SetProperty(cx, rglobal.handle().into(), "______fsInternal______\0".as_ptr() as *const i8, undefined.handle().into());
			}
		}
		false
	}
}
