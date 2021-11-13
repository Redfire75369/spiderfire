/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs;
use std::os;
use std::path::Path;

use mozjs::jsapi::{JS_DefineFunctions, JS_NewPlainObject, JSFunctionSpec, Value};
use mozjs::jsval::ObjectValue;
use mozjs::typedarray::{CreateWith, Uint8Array};

use ion::{IonContext, IonResult};
use ion::error::IonError;
use ion::functions::arguments::Arguments;
use ion::objects::object::{IonObject, IonRawObject};
use runtime::modules::IonModule;

const FS_SOURCE: &str = include_str!("fs.js");

#[js_fn]
unsafe fn readBinary(cx: IonContext, path_str: String) -> IonResult<IonRawObject> {
	let path = Path::new(&path_str);

	if path.is_file() {
		if let Ok(bytes) = fs::read(&path) {
			rooted!(in(cx) let mut array = JS_NewPlainObject(cx));
			if !Uint8Array::create(cx, CreateWith::Slice(bytes.as_slice()), array.handle_mut()).is_ok() {
				if !Uint8Array::create(cx, CreateWith::Length(0), array.handle_mut()).is_ok() {
					return Err(IonError::Error(String::from("Unable to create Uint8Array")));
				}
			}
			Ok(array.get())
		} else {
			Err(IonError::Error(format!("Could not read file: {}", path_str)))
		}
	} else {
		Err(IonError::Error(format!("File {} does not exist", path_str)))
	}
}

#[js_fn]
unsafe fn readString(path_str: String) -> IonResult<String> {
	let path = Path::new(&path_str);

	if path.is_file() {
		if let Ok(str) = fs::read_to_string(&path) {
			Ok(str)
		} else {
			Err(IonError::Error(format!("Could not read file: {}", path_str)))
		}
	} else {
		Err(IonError::Error(format!("File {} does not exist", path_str)))
	}
}

#[js_fn]
unsafe fn readDir(path_str: String) -> IonResult<Vec<String>> {
	let path = Path::new(&path_str);

	if path.is_dir() {
		if let Ok(dir) = fs::read_dir(path) {
			let entries: Vec<_> = dir.filter_map(|entry| entry.ok()).collect();
			let mut str_entries: Vec<String> = entries.iter().map(|entry| entry.file_name().into_string().unwrap()).collect();
			str_entries.sort();

			Ok(str_entries)
		} else {
			Ok(Vec::new())
		}
	} else {
		Err(IonError::Error(format!("Directory {} does not exist", path_str)))
	}
}

#[js_fn]
unsafe fn write(path_str: String, contents: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if !path.is_dir() {
		Ok(fs::write(path, contents).is_ok())
	} else {
		Err(IonError::Error(format!("Path {} is a directory", path_str)))
	}
}

#[js_fn]
unsafe fn createDir(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if !path.is_file() {
		Ok(fs::create_dir(path).is_ok())
	} else {
		Err(IonError::Error(format!("Path {} is a file", path_str)))
	}
}

#[js_fn]
unsafe fn createDirRecursive(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if !path.is_file() {
		Ok(fs::create_dir_all(path).is_ok())
	} else {
		Err(IonError::Error(format!("Path {} is a file", path_str)))
	}
}

#[js_fn]
unsafe fn removeFile(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if path.is_file() {
		Ok(fs::remove_file(path).is_ok())
	} else {
		Err(IonError::Error(format!("Path {} is not a file", path_str)))
	}
}

#[js_fn]
unsafe fn removeDir(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if path.is_dir() {
		Ok(fs::remove_file(path).is_ok())
	} else {
		Err(IonError::Error(format!("Path {} is not a directory", path_str)))
	}
}

#[js_fn]
unsafe fn removeDirRecursive(path_str: String) -> IonResult<bool> {
	let path = Path::new(&path_str);

	if path.is_dir() {
		Ok(fs::remove_dir_all(path).is_ok())
	} else {
		Err(IonError::Error(format!("Path {} is not a directory", path_str)))
	}
}

#[js_fn]
unsafe fn copy(from_str: String, to_str: String) -> IonResult<bool> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	if !from.is_dir() || !to.is_dir() {
		Ok(fs::copy(from, to).is_ok())
	} else {
		Err(IonError::None)
	}
}

#[js_fn]
unsafe fn rename(from_str: String, to_str: String) -> IonResult<bool> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	if !from.is_dir() || !to.is_dir() {
		Ok(fs::rename(from, to).is_ok())
	} else {
		Err(IonError::None)
	}
}

#[js_fn]
unsafe fn softLink(original_str: String, link_str: String) -> IonResult<bool> {
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
		Err(IonError::Error(String::from("Link already exists")))
	}
}

#[js_fn]
unsafe fn hardLink(original_str: String, link_str: String) -> IonResult<bool> {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	if !link.exists() {
		Ok(fs::hard_link(original, link).is_ok())
	} else {
		Err(IonError::Error(String::from("Link already exists")))
	}
}

const METHODS: &[JSFunctionSpec] = &[
	function_spec!(readBinary, 1),
	function_spec!(readString, 1),
	function_spec!(readDir, 1),
	function_spec!(write, 2),
	function_spec!(createDir, 1),
	function_spec!(createDirRecursive, 1),
	function_spec!(removeFile, 1),
	function_spec!(removeDir, 1),
	function_spec!(removeDirRecursive, 1),
	function_spec!(copy, 2),
	function_spec!(rename, 2),
	function_spec!(softLink, 2),
	function_spec!(hardLink, 2),
	JSFunctionSpec::ZERO,
];

/*
 * TODO: Remove JS Wrapper, Stop Global Scope Pollution, Use CreateEmptyModule and AddModuleExport
 * TODO: Waiting on https://bugzilla.mozilla.org/show_bug.cgi?id=1722802
 */
pub unsafe fn init(cx: IonContext, mut global: IonObject) -> bool {
	let internal_key = String::from("______fsInternal______");
	rooted!(in(cx) let fs_module = JS_NewPlainObject(cx));
	if JS_DefineFunctions(cx, fs_module.handle().into(), METHODS.as_ptr()) {
		if global.define(cx, internal_key, ObjectValue(fs_module.get()), 0) {
			let module = IonModule::compile(cx, "fs", None, FS_SOURCE).unwrap();
			module.register("fs")
		} else {
			false
		}
	} else {
		false
	}
}
