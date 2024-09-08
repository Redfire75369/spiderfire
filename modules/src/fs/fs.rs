/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use futures::stream::StreamExt;
use mozjs::jsapi::JSFunctionSpec;
use std::iter::Iterator;
use std::path::Path;
use std::{fs, io, os};
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
fn read_binary(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);

		match tokio::fs::read(path).await {
			Ok(bytes) => Ok(Uint8ArrayWrapper::from(bytes)),
			Err(err) => Err(read_file_error(&path_str, err)),
		}
	})
}

#[js_fn]
fn read_binary_sync(path_str: String) -> Result<Uint8ArrayWrapper> {
	let path = Path::new(&path_str);

	match fs::read(path) {
		Ok(bytes) => Ok(Uint8ArrayWrapper::from(bytes)),
		Err(err) => Err(read_file_error(&path_str, err)),
	}
}

#[js_fn]
fn read_string(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);

		tokio::fs::read_to_string(path).await.map_err(|err| read_file_error(&path_str, err))
	})
}

#[js_fn]
fn read_string_sync(path_str: String) -> Result<String> {
	let path = Path::new(&path_str);

	fs::read_to_string(path).map_err(|err| read_file_error(&path_str, err))
}

#[js_fn]
fn read_dir(cx: &Context, path_str: String) -> Option<Promise> {
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
fn read_dir_sync(path_str: String) -> Result<Vec<String>> {
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
fn create_dir(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		Ok(tokio::fs::create_dir(path).await.is_ok())
	})
}

#[js_fn]
fn create_dir_sync(path_str: String) -> bool {
	let path = Path::new(&path_str);

	fs::create_dir(path).is_ok()
}

#[js_fn]
fn create_dir_recursive(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);

		Ok(tokio::fs::create_dir_all(path).await.is_ok())
	})
}

#[js_fn]
fn create_dir_recursive_sync(path_str: String) -> bool {
	let path = Path::new(&path_str);

	fs::create_dir_all(path).is_ok()
}

#[js_fn]
fn remove_file(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);
		Ok(tokio::fs::remove_file(path).await.is_ok())
	})
}

#[js_fn]
fn remove_file_sync(path_str: String) -> bool {
	let path = Path::new(&path_str);
	fs::remove_file(path).is_ok()
}

#[js_fn]
fn remove_dir(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);
		Ok(tokio::fs::remove_dir(path).await.is_ok())
	})
}

#[js_fn]
fn remove_dir_sync(path_str: String) -> bool {
	let path = Path::new(&path_str);
	fs::remove_dir(path).is_ok()
}

#[js_fn]
fn remove_dir_recursive(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let path = Path::new(&path_str);
		Ok(tokio::fs::remove_dir_all(path).await.is_ok())
	})
}

#[js_fn]
fn remove_dir_recursive_sync(path_str: String) -> bool {
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
fn copy_sync(from_str: String, to_str: String) -> bool {
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
fn rename_sync(from_str: String, to_str: String) -> bool {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	fs::rename(from, to).is_ok()
}

#[js_fn]
fn soft_link(cx: &Context, original_str: String, link_str: String) -> Option<Promise> {
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
fn soft_link_sync(original_str: String, link_str: String) -> bool {
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
fn hard_link(cx: &Context, original_str: String, link_str: String) -> Option<Promise> {
	future_to_promise::<_, _, Error>(cx, async move {
		let original = Path::new(&original_str);
		let link = Path::new(&link_str);

		Ok(tokio::fs::hard_link(original, link).await.is_ok())
	})
}

#[js_fn]
fn hard_link_sync(original_str: String, link_str: String) -> bool {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	fs::hard_link(original, link).is_ok()
}

const SYNC_FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(read_binary_sync, "readBinary", 1),
	function_spec!(read_string_sync, "readString", 1),
	function_spec!(read_dir_sync, "readDir", 1),
	function_spec!(write_sync, "write", 2),
	function_spec!(create_dir_sync, "createDir", 1),
	function_spec!(create_dir_recursive_sync, "createDirRecursive", 1),
	function_spec!(remove_file_sync, "removeFile", 1),
	function_spec!(remove_dir_sync, "removeDir", 1),
	function_spec!(remove_dir_recursive_sync, "removeDirRecursive", 1),
	function_spec!(copy_sync, "copy", 2),
	function_spec!(rename_sync, "rename", 2),
	function_spec!(soft_link_sync, "softLink", 2),
	function_spec!(hard_link_sync, "hardLink", 2),
	JSFunctionSpec::ZERO,
];

const ASYNC_FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(read_binary, "readBinary", 1),
	function_spec!(read_string, "readString", 1),
	function_spec!(read_dir, "readDir", 1),
	function_spec!(write, 2),
	function_spec!(create_dir, "createDir", 1),
	function_spec!(create_dir_recursive, "createDirRecursive", 1),
	function_spec!(remove_file, "removeFile", 1),
	function_spec!(remove_dir, "removeDir", 1),
	function_spec!(remove_dir_recursive, "removeDirRecursive", 1),
	function_spec!(copy, 2),
	function_spec!(rename, 2),
	function_spec!(soft_link, "softLink", 2),
	function_spec!(hard_link, "hardLink", 2),
	JSFunctionSpec::ZERO,
];

#[derive(Default)]
pub struct FileSystemSync;

impl NativeModule for FileSystemSync {
	const NAME: &'static str = "fs/sync";
	const VARIABLE_NAME: &'static str = "fsSync";
	const SOURCE: &'static str = include_str!("fs_sync.js");

	fn module(cx: &Context) -> Option<Object> {
		let fs = Object::new(cx);

		if unsafe { fs.define_methods(cx, SYNC_FUNCTIONS) } {
			Some(fs)
		} else {
			None
		}
	}
}

#[derive(Default)]
pub struct FileSystem;

impl NativeModule for FileSystem {
	const NAME: &'static str = "fs";
	const VARIABLE_NAME: &'static str = "fs";
	const SOURCE: &'static str = include_str!("fs.js");

	fn module(cx: &Context) -> Option<Object> {
		FileSystemSync::module(cx).and_then(|sync| {
			let fs = Object::new(cx);

			if unsafe { fs.define_methods(cx, ASYNC_FUNCTIONS) }
				&& fs.define_as(cx, "sync", &sync, PropertyFlags::CONSTANT_ENUMERATED)
			{
				Some(fs)
			} else {
				None
			}
		})
	}
}
