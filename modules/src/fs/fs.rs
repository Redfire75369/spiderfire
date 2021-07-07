/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::ffi::CString;
use ::std::fs;
use ::std::os;
use ::std::path::Path;
use ::std::ptr;

use mozjs::jsapi::*;
use mozjs::jsval::{ObjectValue, UndefinedValue};
use mozjs::typedarray::{CreateWith, Uint8Array};

use ion::functions::arguments::Arguments;
use ion::functions::macros::{IonContext, IonResult};
use ion::functions::specs::{create_function_spec, NULL_SPEC};
use ion::objects::object::IonRawObject;
use runtime::modules::{compile_module, register_module};

const FS_SOURCE: &str = include_str!("fs.js");

#[js_fn]
unsafe fn read_binary(cx: IonContext, path_str: String) -> IonResult<IonRawObject> {
	let path = Path::new(&path_str);

	if path.is_file() {
		if let Ok(bytes) = fs::read(&path) {
			rooted!(in(cx) let mut obj = JS_NewPlainObject(cx));
			if !Uint8Array::create(cx, CreateWith::Slice(bytes.as_slice()), obj.handle_mut()).is_ok() {
				if !Uint8Array::create(cx, CreateWith::Length(0), obj.handle_mut()).is_ok() {
					return Err(Some(String::from("Unable to create Uint8Array")));
				}
			}
			Ok(obj.get())
		} else {
			Err(Some(String::from(format!("Could not read file: {}", path_str))))
		}
	} else {
		Err(Some(String::from(format!("File {} does not exist", path_str))))
	}
}

#[js_fn]
unsafe fn read_string(path_str: String) -> IonResult<String> {
	let path = Path::new(&path_str);

	if path.is_file() {
		if let Ok(str) = fs::read_to_string(&path) {
			Ok(str)
		} else {
			Err(Some(String::from(format!("Could not read file: {}", path_str))))
		}
	} else {
		Err(Some(String::from(format!("File {} does not exist", path_str))))
	}
}

#[js_fn]
unsafe fn read_dir(cx: IonContext, path_str: String) -> IonResult<IonRawObject> {
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

			Ok(NewArrayObject(cx, &HandleValueArray::from_rooted_slice(dir_entries.as_slice())))
		} else {
			Ok(NewArrayObject(cx, &HandleValueArray::from_rooted_slice(&[])))
		}
	} else {
		Err(Some(String::from(format!("Directory {} does not exist", path_str))))
	}
}

#[js_fn]
unsafe fn write(path_str: String, contents: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if !path.is_dir() {
		Ok(fs::write(path, contents).is_ok())
	} else {
		Err(Some(String::from(format!("Path {} is a directory", path_str))))
	}
}

#[js_fn]
unsafe fn create_dir(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if !path.is_file() {
		Ok(fs::create_dir(path).is_ok())
	} else {
		Err(Some(String::from(format!("Path {} is a file", path_str))))
	}
}

#[js_fn]
unsafe fn create_dir_recursive(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if !path.is_file() {
		Ok(fs::create_dir_all(path).is_ok())
	} else {
		Err(Some(String::from(format!("Path {} is a file", path_str))))
	}
}

#[js_fn]
unsafe fn remove_file(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if path.is_file() {
		Ok(fs::remove_file(path).is_ok())
	} else {
		Err(Some(String::from(format!("Path {} is not a file", path_str))))
	}
}

#[js_fn]
unsafe fn remove_dir(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if path.is_dir() {
		Ok(fs::remove_file(path).is_ok())
	} else {
		Err(Some(String::from(format!("Path {} is not a directory", path_str))))
	}
}

#[js_fn]
unsafe fn remove_dir_recursive(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if path.is_dir() {
		Ok(fs::remove_dir_all(path).is_ok())
	} else {
		Err(Some(String::from(format!("Path {} is not a directory", path_str))))
	}
}

#[js_fn]
unsafe fn copy(from_str: String, to_str: String) -> IonResult<bool> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	if !from.is_dir() || !to.is_dir() {
		Ok(fs::copy(from, to).is_ok())
	} else {
		Err(Some(String::from("")))
	}
}

#[js_fn]
unsafe fn rename(from_str: String, to_str: String) -> IonResult<bool> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	if !from.is_dir() || !to.is_dir() {
		Ok(fs::rename(from, to).is_ok())
	} else {
		Err(Some(String::from("")))
	}
}

#[js_fn]
unsafe fn soft_link(original_str: String, link_str: String) -> IonResult<bool> {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	if !link.exists() {
		#[cfg(target_family = "unix")]
		{
			Ok(os::unix::fs::symlink(original, link).is_ok())
		}
		#[cfg(target_family = "windows")]
		{
			if original.is_file() {
				Ok(os::windows::fs::symlink_file(original, link).is_ok())
			} else if original.is_dir() {
				Ok(os::windows::fs::symlink_dir(original, link).is_ok())
			} else {
				Ok(false)
			}
		}
	} else {
		Err(Some(String::from("Link already exists")))
	}
}

#[js_fn]
unsafe fn hard_link(original_str: String, link_str: String) -> IonResult<bool> {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	if !link.exists() {
		Ok(fs::hard_link(original, link).is_ok())
	} else {
		Err(Some(String::from("Link already exists")))
	}
}

const METHODS: &[JSFunctionSpec] = &[
	create_function_spec(
		"readBinary\0",
		JSNativeWrapper {
			op: Some(read_binary),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"readString\0",
		JSNativeWrapper {
			op: Some(read_string),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"readDir\0",
		JSNativeWrapper {
			op: Some(read_dir),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"write\0",
		JSNativeWrapper {
			op: Some(write),
			info: ptr::null_mut(),
		},
		2,
	),
	create_function_spec(
		"createDir\0",
		JSNativeWrapper {
			op: Some(create_dir),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"createDirRecursive\0",
		JSNativeWrapper {
			op: Some(create_dir_recursive),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"removeFile\0",
		JSNativeWrapper {
			op: Some(remove_file),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"removeDir\0",
		JSNativeWrapper {
			op: Some(remove_dir),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"removeDirRecursive\0",
		JSNativeWrapper {
			op: Some(remove_dir_recursive),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"copy\0",
		JSNativeWrapper {
			op: Some(copy),
			info: ptr::null_mut(),
		},
		2,
	),
	create_function_spec(
		"rename\0",
		JSNativeWrapper {
			op: Some(rename),
			info: ptr::null_mut(),
		},
		2,
	),
	create_function_spec(
		"softLink\0",
		JSNativeWrapper {
			op: Some(soft_link),
			info: ptr::null_mut(),
		},
		1,
	),
	create_function_spec(
		"hardLink\0",
		JSNativeWrapper {
			op: Some(soft_link),
			info: ptr::null_mut(),
		},
		1,
	),
	NULL_SPEC,
];

pub fn init_fs(cx: *mut JSContext, global: *mut JSObject) -> bool {
	unsafe {
		rooted!(in(cx) let fs_module = JS_NewPlainObject(cx));
		rooted!(in(cx) let rglobal = global);
		if JS_DefineFunctions(cx, fs_module.handle().into(), METHODS.as_ptr()) {
			rooted!(in(cx) let fs_module_obj = ObjectValue(fs_module.get()));
			if JS_DefineProperty(
				cx,
				rglobal.handle().into(),
				"______fsInternal______\0".as_ptr() as *const i8,
				fs_module_obj.handle().into(),
				0,
			) {
				return register_module(
					cx,
					&String::from("fs"),
					compile_module(cx, &String::from("fs"), None, &String::from(FS_SOURCE)).unwrap(),
				);
				// && JS_SetProperty(cx, rglobal.handle().into(), "______fsInternal______\0".as_ptr() as *const i8, undefined.handle().into());
			}
		}
		false
	}
}
