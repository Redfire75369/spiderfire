/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::{Path, PathBuf};

use mozjs::jsapi::JSFunctionSpec;

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
fn join(#[ion(varargs)] segments: Vec<String>) -> String {
	let mut path = PathBuf::new();
	for segment in segments {
		path.push(segment);
	}

	String::from(path.to_str().unwrap())
}

#[js_fn]
fn stripPrefix(path: String, prefix: String) -> Result<String> {
	let path = Path::new(&path);

	if let Ok(path) = path.strip_prefix(&prefix) {
		Ok(String::from(path.to_str().unwrap()))
	} else {
		Err(Error::new("Failed to strip prefix from path.", None))
	}
}

#[js_fn]
fn fileStem(path: String) -> Option<String> {
	let path = Path::new(&path);
	path.file_stem().map(|s| String::from(s.to_str().unwrap()))
}

#[js_fn]
fn parent(path: String) -> Option<String> {
	let path = Path::new(&path);
	path.parent().map(|s| String::from(s.to_str().unwrap()))
}

#[js_fn]
fn fileName(path: String) -> Option<String> {
	let path = Path::new(&path);
	path.file_name().map(|s| String::from(s.to_str().unwrap()))
}

#[js_fn]
fn extension(path: String) -> Option<String> {
	let path = Path::new(&path);
	path.extension().map(|s| String::from(s.to_str().unwrap()))
}

#[js_fn]
fn withFileName(path: String, file_name: String) -> String {
	let path = Path::new(&path);
	String::from(path.with_file_name(&file_name).to_str().unwrap())
}

#[js_fn]
fn withExtension(path: String, extension: String) -> String {
	let path = Path::new(&path);
	String::from(path.with_extension(&extension).to_str().unwrap())
}

#[js_fn]
fn isAbsolute(path: String) -> bool {
	Path::new(&path).is_absolute()
}

#[js_fn]
fn isRelative(path: String) -> bool {
	Path::new(&path).is_relative()
}

#[js_fn]
fn hasRoot(path: String) -> bool {
	Path::new(&path).has_root()
}

#[js_fn]
fn startsWith(path: String, prefix: String) -> bool {
	Path::new(&path).starts_with(&prefix)
}

#[js_fn]
fn endsWith(path: String, prefix: String) -> bool {
	Path::new(&path).ends_with(&prefix)
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

	fn module<'cx>(cx: &'cx Context) -> Option<Object<'cx>> {
		let mut path = Object::new(cx);
		if path.define_methods(cx, FUNCTIONS)
			&& path.define_as(cx, "separator", String::from(SEPARATOR), PropertyFlags::CONSTANT_ENUMERATED)
			&& path.define_as(cx, "delimiter", String::from(DELIMITER), PropertyFlags::CONSTANT_ENUMERATED)
		{
			return Some(path);
		}
		None
	}
}
