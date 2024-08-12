/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{fs, os};
use std::iter::Iterator;
use std::path::Path;

use futures::stream::StreamExt;
use mozjs::jsapi::JSFunctionSpec;
use tokio_stream::wrappers::ReadDirStream;

use ion::{Context, Error, Object, Promise, Result};
use ion::flags::PropertyFlags;
use ion::typedarray::Uint8ArrayWrapper;
use runtime::module::NativeModule;
use runtime::promise::future_to_promise;

fn check_exists(path: &Path) -> Result<()> {
	if path.exists() {
		Ok(())
	} else {
		Err(Error::new(
			format!("Path {} does not exist", path.to_str().unwrap()),
			None,
		))
	}
}

fn check_not_exists(path: &Path) -> Result<()> {
	if !path.exists() {
		Ok(())
	} else {
		Err(Error::new(format!("Path {} exists", path.to_str().unwrap()), None))
	}
}

fn check_is_file(path: &Path) -> Result<()> {
	check_exists(path)?;
	if path.is_file() {
		Ok(())
	} else {
		Err(Error::new(
			format!("Path {} is not a file", path.to_str().unwrap()),
			None,
		))
	}
}

fn check_is_not_file(path: &Path) -> Result<()> {
	check_exists(path)?;
	if path.is_file() {
		Err(Error::new(format!("Path {} is a file", path.to_str().unwrap()), None))
	} else {
		Ok(())
	}
}

fn check_is_dir(path: &Path) -> Result<()> {
	check_exists(path)?;
	if path.is_dir() {
		Ok(())
	} else {
		Err(Error::new(
			format!("Path {} is not a directory", path.to_str().unwrap()),
			None,
		))
	}
}

fn check_is_not_dir(path: &Path) -> Result<()> {
	check_exists(path)?;
	if path.is_dir() {
		Err(Error::new(
			format!("Path {} is a directory", path.to_str().unwrap()),
			None,
		))
	} else {
		Ok(())
	}
}

#[js_fn]
fn readBinary(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);

		check_is_file(path)?;
		if let Ok(bytes) = tokio::fs::read(&path).await {
			Ok(Uint8ArrayWrapper::from(bytes))
		} else {
			Err(Error::new(format!("Could not read file: {}", path_str), None))
		}
	})
}

#[js_fn]
fn readBinarySync(path_str: String) -> Result<Uint8ArrayWrapper> {
	let path = Path::new(&path_str);

	check_is_file(path)?;
	if let Ok(bytes) = fs::read(path) {
		Ok(Uint8ArrayWrapper::from(bytes))
	} else {
		Err(Error::new(format!("Could not read file: {}", path_str), None))
	}
}

#[js_fn]
fn readString(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);

		check_is_file(path)?;
		if let Ok(str) = tokio::fs::read_to_string(path).await {
			Ok(str)
		} else {
			Err(Error::new(format!("Could not read file: {}", path_str), None))
		}
	})
}

#[js_fn]
fn readStringSync(path_str: String) -> Result<String> {
	let path = Path::new(&path_str);

	check_is_file(path)?;
	if let Ok(str) = fs::read_to_string(path) {
		Ok(str)
	} else {
		Err(Error::new(format!("Could not read file: {}", path_str), None))
	}
}

#[js_fn]
fn readDir(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		check_is_dir(path)?;
		if let Ok(dir) = tokio::fs::read_dir(path).await {
			let mut entries: Vec<_> = ReadDirStream::new(dir)
				.filter_map(|entry| async move { entry.ok() })
				.map(|entry| entry.file_name().into_string().unwrap())
				.collect()
				.await;
			entries.sort();

			Ok(entries)
		} else {
			Ok(Vec::new())
		}
	})
}

#[js_fn]
fn readDirSync(path_str: String) -> Result<Vec<String>> {
	let path = Path::new(&path_str);

	check_is_dir(path)?;
	if let Ok(dir) = fs::read_dir(path) {
		let mut entries: Vec<_> = dir
			.filter_map(|entry| entry.ok())
			.map(|entry| entry.file_name().into_string().unwrap())
			.collect();
		entries.sort();

		Ok(entries)
	} else {
		Ok(Vec::new())
	}
}

#[js_fn]
fn write(cx: &Context, path_str: String, contents: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		check_is_not_dir(path)?;
		Ok(tokio::fs::write(path, contents).await.is_ok())
	})
}

#[js_fn]
fn writeSync(path_str: String, contents: String) -> Result<bool> {
	let path = Path::new(&path_str);

	check_is_not_dir(path)?;
	Ok(fs::write(path, contents).is_ok())
}

#[js_fn]
fn createDir(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		check_is_not_file(path)?;
		Ok(tokio::fs::create_dir(path).await.is_ok())
	})
}

#[js_fn]
fn createDirSync(path_str: String) -> Result<bool> {
	let path = Path::new(&path_str);

	check_is_not_file(path)?;
	Ok(fs::create_dir(path).is_ok())
}

#[js_fn]
fn createDirRecursive(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		check_is_not_file(path)?;
		Ok(tokio::fs::create_dir_all(path).await.is_ok())
	})
}

#[js_fn]
fn createDirRecursiveSync(path_str: String) -> Result<bool> {
	let path = Path::new(&path_str);

	check_is_not_file(path)?;
	Ok(fs::create_dir_all(path).is_ok())
}

#[js_fn]
fn removeFile(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		check_is_file(path)?;
		Ok(tokio::fs::remove_file(path).await.is_ok())
	})
}

#[js_fn]
fn removeFileSync(path_str: String) -> Result<bool> {
	let path = Path::new(&path_str);

	check_is_file(path)?;
	Ok(fs::remove_file(path).is_ok())
}

#[js_fn]
fn removeDir(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		check_is_dir(path)?;
		Ok(tokio::fs::remove_file(path).await.is_ok())
	})
}

#[js_fn]
fn removeDirSync(path_str: String) -> Result<bool> {
	let path = Path::new(&path_str);

	check_is_dir(path)?;
	Ok(fs::remove_file(path).is_ok())
}

#[js_fn]
fn removeDirRecursive(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		check_is_dir(path)?;
		Ok(tokio::fs::remove_dir_all(path).await.is_ok())
	})
}

#[js_fn]
fn removeDirRecursiveSync(path_str: String) -> Result<bool> {
	let path = Path::new(&path_str);

	check_is_dir(path)?;
	Ok(fs::remove_dir_all(path).is_ok())
}

#[js_fn]
fn copy(cx: &Context, from_str: String, to_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let from = Path::new(&from_str);
		let to = Path::new(&to_str);

		check_is_not_dir(from)?;
		check_is_not_dir(to)?;
		Ok(tokio::fs::copy(from, to).await.is_ok())
	})
}

#[js_fn]
fn copySync(from_str: String, to_str: String) -> Result<bool> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	check_is_not_dir(from)?;
	check_is_not_dir(to)?;
	Ok(fs::copy(from, to).is_ok())
}

#[js_fn]
fn rename(cx: &Context, from_str: String, to_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let from = Path::new(&from_str);
		let to = Path::new(&to_str);

		check_is_not_dir(from)?;
		check_is_not_dir(to)?;
		Ok(tokio::fs::rename(from, to).await.is_ok())
	})
}

#[js_fn]
fn renameSync(from_str: String, to_str: String) -> Result<bool> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	check_is_not_dir(from)?;
	check_is_not_dir(to)?;
	Ok(fs::rename(from, to).is_ok())
}

#[js_fn]
fn softLink(cx: &Context, original_str: String, link_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let original = Path::new(&original_str);
		let link = Path::new(&link_str);

		check_not_exists(link)?;
		#[cfg(target_family = "unix")]
		{
			Ok(tokio::fs::symlink(original, link).await.is_ok())
		}
		#[cfg(target_family = "windows")]
		{
			if original.is_file() {
				Ok(tokio::fs::symlink_file(original, link).await.is_ok())
			} else if original.is_dir() {
				Ok(tokio::fs::symlink_dir(original, link).await.is_ok())
			} else {
				Ok(false)
			}
		}
	})
}

#[js_fn]
fn softLinkSync(original_str: String, link_str: String) -> Result<bool> {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	check_not_exists(link)?;
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
}

#[js_fn]
fn hardLink(cx: &Context, original_str: String, link_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let original = Path::new(&original_str);
		let link = Path::new(&link_str);

		check_not_exists(link)?;
		Ok(tokio::fs::hard_link(original, link).await.is_ok())
	})
}

#[js_fn]
fn hardLinkSync(original_str: String, link_str: String) -> Result<bool> {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	check_not_exists(link)?;
	Ok(fs::hard_link(original, link).is_ok())
}

const SYNC_FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(readBinarySync, "readBinary", 1),
	function_spec!(readStringSync, "readString", 1),
	function_spec!(readDirSync, "readDir", 1),
	function_spec!(writeSync, "write", 2),
	function_spec!(createDirSync, "createDir", 1),
	function_spec!(createDirRecursiveSync, "createDirRecursive", 1),
	function_spec!(removeFileSync, "removeFile", 1),
	function_spec!(removeDirSync, "removeDir", 1),
	function_spec!(removeDirRecursiveSync, "removeDirRecursive", 1),
	function_spec!(copySync, "copy", 2),
	function_spec!(renameSync, "rename", 2),
	function_spec!(softLinkSync, "softLink", 2),
	function_spec!(hardLinkSync, "hardLink", 2),
	JSFunctionSpec::ZERO,
];

const ASYNC_FUNCTIONS: &[JSFunctionSpec] = &[
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

#[derive(Default)]
pub struct FileSystem;

impl NativeModule for FileSystem {
	const NAME: &'static str = "fs";
	const SOURCE: &'static str = include_str!("fs.js");

	fn module(cx: &Context) -> Option<Object> {
		let fs = Object::new(cx);
		let sync = Object::new(cx);

		if unsafe { fs.define_methods(cx, ASYNC_FUNCTIONS) }
			&& unsafe { sync.define_methods(cx, SYNC_FUNCTIONS) }
			&& fs.define_as(cx, "sync", &sync, PropertyFlags::CONSTANT_ENUMERATED)
		{
			return Some(fs);
		}
		None
	}
}
