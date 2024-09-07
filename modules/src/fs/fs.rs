/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::iter::Iterator;
use std::path::Path;
use std::{fs, io, os};

use futures::stream::StreamExt;
use mozjs::jsapi::JSFunctionSpec;
use tokio_stream::wrappers::ReadDirStream;

use ion::flags::PropertyFlags;
use ion::typedarray::Uint8ArrayWrapper;
use ion::{Context, Error, Object, Promise, Result};
use runtime::globals::file::BufferSource;
use runtime::module::NativeModule;
use runtime::promise::future_to_promise;

fn read_file_error(path: &str, err: io::Error) -> Error {
	Error::new(format!("Could not read file: {}\n{}", path, err), None)
}

fn read_dir_error(path: &str, err: io::Error) -> Error {
	Error::new(format!("Could not read directory: {}\n{}", path, err), None)
}

#[js_fn]
fn readBinary(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);

		match tokio::fs::read(path).await {
			Ok(bytes) => Ok(Uint8ArrayWrapper::from(bytes)),
			Err(err) => Err(read_file_error(&path_str, err)),
		}
	})
}

#[js_fn]
fn readBinarySync(path_str: String) -> Result<Uint8ArrayWrapper> {
	let path = Path::new(&path_str);

	match fs::read(path) {
		Ok(bytes) => Ok(Uint8ArrayWrapper::from(bytes)),
		Err(err) => Err(read_file_error(&path_str, err)),
	}
}

#[js_fn]
fn readString(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);

		tokio::fs::read_to_string(path).await.map_err(|err| read_file_error(&path_str, err))
	})
}

#[js_fn]
fn readStringSync(path_str: String) -> Result<String> {
	let path = Path::new(&path_str);

	fs::read_to_string(path).map_err(|err| read_file_error(&path_str, err))
}

#[js_fn]
fn readDir(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		match tokio::fs::read_dir(path).await {
			Ok(dir) => {
				let mut entries: Vec<_> = ReadDirStream::new(dir)
					.filter_map(|entry| async move { entry.ok() })
					.map(|entry| entry.file_name().into_string().unwrap())
					.collect()
					.await;
				entries.sort();

				Ok(entries)
			}
			Err(err) => Err(read_dir_error(&path_str, err)),
		}
	})
}

#[js_fn]
fn readDirSync(path_str: String) -> Result<Vec<String>> {
	let path = Path::new(&path_str);

	match fs::read_dir(path) {
		Ok(dir) => {
			let mut entries: Vec<_> = dir
				.filter_map(|entry| entry.ok())
				.map(|entry| entry.file_name().into_string().unwrap())
				.collect();
			entries.sort();

			Ok(entries)
		}
		Err(err) => Err(read_dir_error(&path_str, err)),
	}
}

#[js_fn]
fn write<'cx>(
	cx: &'cx Context, path_str: String, #[ion(convert = false)] contents: BufferSource<'cx>,
) -> Option<Promise<'cx>> {
	let contents = contents.to_vec();
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);
		Ok(tokio::fs::write(path, contents).await.is_ok())
	})
}

#[js_fn]
fn write_sync(path_str: String, #[ion(convert = false)] contents: BufferSource) -> bool {
	let path = Path::new(&path_str);

	let contents = unsafe { contents.as_slice() };
	fs::write(path, contents).is_ok()
}

#[js_fn]
fn createDir(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		Ok(tokio::fs::create_dir(path).await.is_ok())
	})
}

#[js_fn]
fn createDirSync(path_str: String) -> bool {
	let path = Path::new(&path_str);

	fs::create_dir(path).is_ok()
}

#[js_fn]
fn createDirRecursive(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		Ok(tokio::fs::create_dir_all(path).await.is_ok())
	})
}

#[js_fn]
fn createDirRecursiveSync(path_str: String) -> bool {
	let path = Path::new(&path_str);

	fs::create_dir_all(path).is_ok()
}

#[js_fn]
fn removeFile(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);
		Ok(tokio::fs::remove_file(path).await.is_ok())
	})
}

#[js_fn]
fn removeFileSync(path_str: String) -> bool {
	let path = Path::new(&path_str);
	fs::remove_file(path).is_ok()
}

#[js_fn]
fn removeDir(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);
		Ok(tokio::fs::remove_dir(path).await.is_ok())
	})
}

#[js_fn]
fn removeDirSync(path_str: String) -> bool {
	let path = Path::new(&path_str);
	fs::remove_dir(path).is_ok()
}

#[js_fn]
fn removeDirRecursive(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);
		Ok(tokio::fs::remove_dir_all(path).await.is_ok())
	})
}

#[js_fn]
fn removeDirRecursiveSync(path_str: String) -> bool {
	let path = Path::new(&path_str);
	fs::remove_dir_all(path).is_ok()
}

#[js_fn]
fn copy(cx: &Context, from_str: String, to_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let from = Path::new(&from_str);
		let to = Path::new(&to_str);

		Ok(tokio::fs::copy(from, to).await.is_ok())
	})
}

#[js_fn]
fn copySync(from_str: String, to_str: String) -> bool {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	fs::copy(from, to).is_ok()
}

#[js_fn]
fn rename(cx: &Context, from_str: String, to_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let from = Path::new(&from_str);
		let to = Path::new(&to_str);

		Ok(tokio::fs::rename(from, to).await.is_ok())
	})
}

#[js_fn]
fn renameSync(from_str: String, to_str: String) -> bool {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	fs::rename(from, to).is_ok()
}

#[js_fn]
fn softLink(cx: &Context, original_str: String, link_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let original = Path::new(&original_str);
		let link = Path::new(&link_str);

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
fn softLinkSync(original_str: String, link_str: String) -> bool {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	#[cfg(target_family = "unix")]
	{
		os::unix::fs::symlink(original, link).is_ok()
	}
	#[cfg(target_family = "windows")]
	{
		if original.is_file() {
			os::windows::fs::symlink_file(original, link).is_ok()
		} else if original.is_dir() {
			os::windows::fs::symlink_dir(original, link).is_ok()
		} else {
			false
		}
	}
}

#[js_fn]
fn hardLink(cx: &Context, original_str: String, link_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let original = Path::new(&original_str);
		let link = Path::new(&link_str);

		Ok(tokio::fs::hard_link(original, link).await.is_ok())
	})
}

#[js_fn]
fn hardLinkSync(original_str: String, link_str: String) -> bool {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	fs::hard_link(original, link).is_ok()
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
