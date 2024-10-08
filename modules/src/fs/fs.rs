/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::iter::Iterator;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::Path;
use std::{fs, os};

use futures::stream::StreamExt;
use ion::class::ClassObjectWrapper;
use ion::flags::PropertyFlags;
use ion::function::Opt;
use ion::{ClassDefinition, Context, Object, Promise, Result};
use mozjs::jsapi::{JSFunctionSpec, JSObject};
use runtime::module::NativeModule;
use runtime::promise::future_to_promise;
use tokio_stream::wrappers::ReadDirStream;
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_FLAGS_AND_ATTRIBUTES};

use crate::fs::{dir_error, file_error, remove_error, translate_error, FileHandle};

#[derive(Copy, Clone, Debug, FromValue)]
struct OpenOptions {
	#[ion(default = true)]
	read: bool,
	#[ion(default)]
	write: bool,
	#[ion(default)]
	append: bool,
	#[ion(default)]
	truncate: bool,
	#[ion(default)]
	create: bool,
	#[ion(name = "createNew", default)]
	create_new: bool,
}

impl OpenOptions {
	fn into_std(self) -> fs::OpenOptions {
		let mut options = fs::OpenOptions::new();

		options
			.read(self.read)
			.write(self.write)
			.append(self.append)
			.truncate(self.truncate)
			.create(self.create)
			.create_new(self.create_new);

		options
	}

	fn into_tokio(self) -> tokio::fs::OpenOptions {
		let options = self.into_std();
		tokio::fs::OpenOptions::from(options)
	}
}

impl Default for OpenOptions {
	fn default() -> OpenOptions {
		OpenOptions {
			read: true,
			write: false,
			append: false,
			truncate: false,
			create: false,
			create_new: false,
		}
	}
}

#[js_fn]
fn open(cx: &Context, path_str: String, Opt(options): Opt<OpenOptions>) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);
		let options = options.unwrap_or_default().into_tokio();

		match options.open(path).await {
			Ok(file) => Ok(ClassObjectWrapper(Box::new(FileHandle::new(
				&path_str,
				file.into_std().await,
			)))),
			Err(err) => Err(file_error("open", &path_str, err)),
		}
	})
}

#[js_fn]
fn open_sync(cx: &Context, path_str: String, Opt(options): Opt<OpenOptions>) -> Result<*mut JSObject> {
	let path = Path::new(&path_str);
	let options = options.unwrap_or_default().into_std();

	match options.open(path) {
		Ok(file) => Ok(FileHandle::new_object(cx, Box::new(FileHandle::new(&path_str, file)))),
		Err(err) => Err(file_error("open", &path_str, err)),
	}
}

#[js_fn]
fn create(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);
		let mut options = tokio::fs::OpenOptions::new();
		options.read(true).write(true).truncate(true).create(true);

		match options.open(path).await {
			Ok(file) => Ok(ClassObjectWrapper(Box::new(FileHandle::new(
				&path_str,
				file.into_std().await,
			)))),
			Err(err) => Err(file_error("create", &path_str, err)),
		}
	})
}

#[js_fn]
fn create_sync(cx: &Context, path_str: String) -> Result<*mut JSObject> {
	let path = Path::new(&path_str);
	let mut options = fs::OpenOptions::new();
	options.read(true).write(true).truncate(true).create(true);

	match options.open(path) {
		Ok(file) => Ok(FileHandle::new_object(cx, Box::new(FileHandle::new(&path_str, file)))),
		Err(err) => Err(file_error("create", &path_str, err)),
	}
}

#[js_fn]
fn read_dir(cx: &Context, path_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
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
			Err(err) => Err(dir_error("read", &path_str, err)),
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
		Err(err) => Err(dir_error("read", &path_str, err)),
	}
}

#[js_fn]
fn create_dir(cx: &Context, path_str: String, Opt(recursive): Opt<bool>) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);
		let recursive = recursive.unwrap_or_default();

		let result = if recursive {
			tokio::fs::create_dir_all(path).await
		} else {
			tokio::fs::create_dir(path).await
		};
		match result {
			Ok(_) => Ok(()),
			Err(err) => Err(dir_error("create", &path_str, err)),
		}
	})
}

#[js_fn]
fn create_dir_sync(path_str: String, Opt(recursive): Opt<bool>) -> Result<()> {
	let path = Path::new(&path_str);
	let recursive = recursive.unwrap_or_default();

	let result = if recursive {
		fs::create_dir_all(path)
	} else {
		fs::create_dir(path)
	};
	match result {
		Ok(_) => Ok(()),
		Err(err) => Err(dir_error("create", &path_str, err)),
	}
}

#[js_fn]
fn remove(cx: &Context, path_str: String, Opt(recursive): Opt<bool>) -> Option<Promise> {
	future_to_promise(cx, async move {
		let path = Path::new(&path_str);
		let recursive = recursive.unwrap_or_default();

		let metadata = tokio::fs::symlink_metadata(path).await?;
		let file_type = metadata.file_type();

		let result = if file_type.is_dir() {
			if recursive {
				tokio::fs::remove_dir_all(path).await
			} else {
				tokio::fs::remove_dir(path).await
			}
		} else {
			#[cfg(unix)]
			{
				tokio::fs::remove_file(path).await
			}

			#[cfg(windows)]
			{
				let attributes = FILE_FLAGS_AND_ATTRIBUTES(metadata.file_attributes());
				if attributes.contains(FILE_ATTRIBUTE_DIRECTORY) {
					tokio::fs::remove_dir(path).await
				} else {
					tokio::fs::remove_file(path).await
				}
			}
		};
		result.map_err(|err| remove_error(&path_str, err))
	})
}

#[js_fn]
fn remove_sync(path_str: String, Opt(recursive): Opt<bool>) -> Result<()> {
	let path = Path::new(&path_str);
	let recursive = recursive.unwrap_or_default();

	let metadata = fs::symlink_metadata(path)?;
	let file_type = metadata.file_type();

	let result = if file_type.is_dir() {
		if recursive {
			fs::remove_dir_all(path)
		} else {
			fs::remove_dir(path)
		}
	} else {
		#[cfg(unix)]
		{
			fs::remove_file(path)
		}

		#[cfg(windows)]
		{
			let attributes = FILE_FLAGS_AND_ATTRIBUTES(metadata.file_attributes());
			if attributes.contains(FILE_ATTRIBUTE_DIRECTORY) {
				fs::remove_dir(path)
			} else {
				fs::remove_file(path)
			}
		}
	};
	result.map_err(|err| remove_error(&path_str, err))
}

#[js_fn]
fn copy(cx: &Context, from_str: String, to_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let from = Path::new(&from_str);
		let to = Path::new(&to_str);

		tokio::fs::copy(from, to)
			.await
			.map_err(|err| translate_error("copy from", &from_str, &to_str, err))
	})
}

#[js_fn]
fn copy_sync(from_str: String, to_str: String) -> Result<u64> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	fs::copy(from, to).map_err(|err| translate_error("copy from", &from_str, &to_str, err))
}

#[js_fn]
fn rename(cx: &Context, from_str: String, to_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let from = Path::new(&from_str);
		let to = Path::new(&to_str);

		tokio::fs::rename(from, to)
			.await
			.map_err(|err| translate_error("rename from", &from_str, &to_str, err))
	})
}

#[js_fn]
fn rename_sync(from_str: String, to_str: String) -> Result<()> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	fs::rename(from, to).map_err(|err| translate_error("rename from", &from_str, &to_str, err))
}

#[js_fn]
fn symlink(cx: &Context, original_str: String, link_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let original = Path::new(&original_str);
		let link = Path::new(&link_str);

		let result;
		#[cfg(target_family = "unix")]
		{
			result = tokio::fs::symlink(original, link).await;
		}
		#[cfg(target_family = "windows")]
		{
			result = if original.is_dir() {
				tokio::fs::symlink_dir(original, link).await
			} else {
				tokio::fs::symlink_file(original, link).await
			};
		}
		result.map_err(|err| translate_error("symlink", &original_str, &link_str, err))
	})
}

#[js_fn]
fn symlink_sync(original_str: String, link_str: String) -> Result<()> {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	let result;
	#[cfg(target_family = "unix")]
	{
		result = os::unix::fs::symlink(original, link)
	}
	#[cfg(target_family = "windows")]
	{
		result = if original.is_dir() {
			os::windows::fs::symlink_dir(original, link)
		} else {
			os::windows::fs::symlink_file(original, link)
		};
	}
	result.map_err(|err| translate_error("symlink", &original_str, &link_str, err))
}

#[js_fn]
fn link(cx: &Context, original_str: String, link_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let original = Path::new(&original_str);
		let link = Path::new(&link_str);

		tokio::fs::hard_link(original, link)
			.await
			.map_err(|err| translate_error("link", &original_str, &link_str, err))
	})
}

#[js_fn]
fn link_sync(original_str: String, link_str: String) -> Result<()> {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	fs::hard_link(original, link).map_err(|err| translate_error("link", &original_str, &link_str, err))
}

const SYNC_FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(open_sync, "open", 1),
	function_spec!(create_sync, "create", 1),
	function_spec!(read_dir_sync, "readDir", 1),
	function_spec!(create_dir_sync, "createDir", 1),
	function_spec!(remove_sync, "remove", 1),
	function_spec!(copy_sync, "copy", 2),
	function_spec!(rename_sync, "rename", 2),
	function_spec!(symlink_sync, "symlink", 2),
	function_spec!(link_sync, "link", 2),
	JSFunctionSpec::ZERO,
];

const ASYNC_FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(open, 1),
	function_spec!(create, 1),
	function_spec!(read_dir, "readDir", 1),
	function_spec!(create_dir, "createDir", 1),
	function_spec!(remove, "remove", 1),
	function_spec!(copy, 2),
	function_spec!(rename, 2),
	function_spec!(symlink, "symlink", 2),
	function_spec!(link, "link", 2),
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
				&& FileHandle::init_class(cx, &fs).0
			{
				Some(fs)
			} else {
				None
			}
		})
	}
}
