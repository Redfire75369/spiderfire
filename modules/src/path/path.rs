/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::{Path, PathBuf};

use mozjs::jsapi::{JS_DefineFunctions, JS_NewPlainObject, JSFunctionSpec};

use ion::{IonContext, IonResult};
use ion::error::IonError;
use ion::functions::arguments::Arguments;
use ion::objects::object::IonObject;
use runtime::modules::IonModule;

const PATH_SOURCE: &str = include_str!("path.js");

#[cfg(windows)]
const SEPARATOR: &str = "\\";
#[cfg(unix)]
const SEPARATOR: &str = "/";

#[cfg(windows)]
const DELIMITER: &str = ";";
#[cfg(unix)]
const DELIMITER: &str = ":";

#[js_fn]
unsafe fn join(#[varargs] segments: Vec<String>) -> IonResult<String> {
	let mut path = PathBuf::new();
	for segment in segments {
		path.push(segment);
	}

	Ok(String::from(path.to_str().unwrap()))
}

#[js_fn]
unsafe fn stripPrefix(path: String, prefix: String) -> IonResult<String> {
	let path = Path::new(&path);

	if let Ok(path) = path.strip_prefix(&prefix) {
		Ok(String::from(path.to_str().unwrap()))
	} else {
		Err(IonError::Error(String::from("Failed to strip prefix from path.")))
	}
}

#[js_fn]
unsafe fn fileStem(path: String) -> IonResult<Option<String>> {
	let path = Path::new(&path);
	Ok(path.file_stem().map(|s| String::from(s.to_str().unwrap())))
}

#[js_fn]
unsafe fn parent(path: String) -> IonResult<Option<String>> {
	let path = Path::new(&path);
	Ok(path.parent().map(|s| String::from(s.to_str().unwrap())))
}

#[js_fn]
unsafe fn fileName(path: String) -> IonResult<Option<String>> {
	let path = Path::new(&path);
	Ok(path.file_name().map(|s| String::from(s.to_str().unwrap())))
}

#[js_fn]
unsafe fn extension(path: String) -> IonResult<Option<String>> {
	let path = Path::new(&path);
	Ok(path.extension().map(|s| String::from(s.to_str().unwrap())))
}

#[js_fn]
unsafe fn withFileName(path: String, file_name: String) -> IonResult<String> {
	let path = Path::new(&path);
	Ok(String::from(path.with_file_name(&file_name).to_str().unwrap()))
}

#[js_fn]
unsafe fn withExtension(path: String, extension: String) -> IonResult<String> {
	let path = Path::new(&path);
	Ok(String::from(path.with_extension(&extension).to_str().unwrap()))
}

#[js_fn]
unsafe fn isAbsolute(path: String) -> IonResult<bool> {
	Ok(Path::new(&path).is_absolute())
}

#[js_fn]
unsafe fn isRelative(path: String) -> IonResult<bool> {
	Ok(Path::new(&path).is_relative())
}

#[js_fn]
unsafe fn hasRoot(path: String) -> IonResult<bool> {
	Ok(Path::new(&path).has_root())
}

#[js_fn]
unsafe fn startsWith(path: String, prefix: String) -> IonResult<bool> {
	Ok(Path::new(&path).starts_with(&prefix))
}

#[js_fn]
unsafe fn endsWith(path: String, prefix: String) -> IonResult<bool> {
	Ok(Path::new(&path).ends_with(&prefix))
}

const METHODS: &[JSFunctionSpec] = &[
	function_spec!(join, 0),
	function_spec!(stripPrefix, 2),
	function_spec!(fileStem, 1),
	function_spec!(parent, 1),
	function_spec!(fileName, 1),
	function_spec!(extension, 1),
	function_spec!(withFileName, 2),
	function_spec!(withExtension, 2),
	function_spec!(isAbsolute, 1),
	function_spec!(isRelative, 1),
	function_spec!(hasRoot, 1),
	function_spec!(startsWith, 2),
	function_spec!(endsWith, 2),
	JSFunctionSpec::ZERO,
];

/*
 * TODO: Remove JS Wrapper, Stop Global Scope Pollution, Use CreateEmptyModule and AddModuleExport
 * TODO: Waiting on https://bugzilla.mozilla.org/show_bug.cgi?id=1722802
 */
pub unsafe fn init(cx: IonContext, mut global: IonObject) -> bool {
	let internal_key = "______pathInternal______";
	rooted!(in(cx) let path_module = JS_NewPlainObject(cx));
	if JS_DefineFunctions(cx, path_module.handle().into(), METHODS.as_ptr()) {
		if IonObject::from(path_module.get()).define_as(cx, "separator", String::from(SEPARATOR), 0)
			&& IonObject::from(path_module.get()).define_as(cx, "delimiter", String::from(DELIMITER), 0)
		{
			if global.define_as(cx, internal_key, path_module.get(), 0) {
				let module = IonModule::compile(cx, "path", None, PATH_SOURCE).unwrap();
				module.register("path")
			} else {
				false
			}
		} else {
			false
		}
	} else {
		false
	}
}
