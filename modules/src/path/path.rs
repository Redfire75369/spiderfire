/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::{Path, PathBuf};

use mozjs::jsapi::{JS_DefineFunctions, JSFunctionSpec};

use ion::{Context, Error, Object, Result};
use ion::flags::PropertyFlags;
use runtime::modules::NativeModule;

#[cfg(windows)]
const SEPARATOR: &str = "\\";
#[cfg(unix)]
const SEPARATOR: &str = "/";

#[cfg(windows)]
const DELIMITER: &str = ";";
#[cfg(unix)]
const DELIMITER: &str = ":";

#[js_fn]
fn join(#[varargs] segments: Vec<String>) -> Result<String> {
	let mut path = PathBuf::new();
	for segment in segments {
		path.push(segment);
	}

	Ok(String::from(path.to_str().unwrap()))
}

#[js_fn]
fn stripPrefix(path: String, prefix: String) -> Result<String> {
	let path = Path::new(&path);

	if let Ok(path) = path.strip_prefix(&prefix) {
		Ok(String::from(path.to_str().unwrap()))
	} else {
		Err(Error::Error(String::from("Failed to strip prefix from path.")))
	}
}

#[js_fn]
fn fileStem(path: String) -> Result<Option<String>> {
	let path = Path::new(&path);
	Ok(path.file_stem().map(|s| String::from(s.to_str().unwrap())))
}

#[js_fn]
fn parent(path: String) -> Result<Option<String>> {
	let path = Path::new(&path);
	Ok(path.parent().map(|s| String::from(s.to_str().unwrap())))
}

#[js_fn]
fn fileName(path: String) -> Result<Option<String>> {
	let path = Path::new(&path);
	Ok(path.file_name().map(|s| String::from(s.to_str().unwrap())))
}

#[js_fn]
fn extension(path: String) -> Result<Option<String>> {
	let path = Path::new(&path);
	Ok(path.extension().map(|s| String::from(s.to_str().unwrap())))
}

#[js_fn]
fn withFileName(path: String, file_name: String) -> Result<String> {
	let path = Path::new(&path);
	Ok(String::from(path.with_file_name(&file_name).to_str().unwrap()))
}

#[js_fn]
fn withExtension(path: String, extension: String) -> Result<String> {
	let path = Path::new(&path);
	Ok(String::from(path.with_extension(&extension).to_str().unwrap()))
}

#[js_fn]
fn isAbsolute(path: String) -> Result<bool> {
	Ok(Path::new(&path).is_absolute())
}

#[js_fn]
fn isRelative(path: String) -> Result<bool> {
	Ok(Path::new(&path).is_relative())
}

#[js_fn]
fn hasRoot(path: String) -> Result<bool> {
	Ok(Path::new(&path).has_root())
}

#[js_fn]
fn startsWith(path: String, prefix: String) -> Result<bool> {
	Ok(Path::new(&path).starts_with(&prefix))
}

#[js_fn]
fn endsWith(path: String, prefix: String) -> Result<bool> {
	Ok(Path::new(&path).ends_with(&prefix))
}

const FUNCTIONS: &[JSFunctionSpec] = &[
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

#[derive(Default)]
pub struct PathM;

impl NativeModule for PathM {
	const NAME: &'static str = "path";
	const SOURCE: &'static str = include_str!("path.js");

	fn module(cx: Context) -> Option<Object> {
		rooted!(in(cx) let path = *Object::new(cx));
		if unsafe { JS_DefineFunctions(cx, path.handle().into(), FUNCTIONS.as_ptr()) }
			&& Object::from(path.get()).define_as(cx, "separator", String::from(SEPARATOR), PropertyFlags::CONSTANT_ENUMERATED)
			&& Object::from(path.get()).define_as(cx, "delimiter", String::from(DELIMITER), PropertyFlags::CONSTANT_ENUMERATED)
		{
			return Some(Object::from(path.get()));
		}
		None
	}
}
