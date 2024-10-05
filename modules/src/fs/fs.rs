/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::fs::File;
use std::future::{poll_fn, Future};
use std::io::{Read, Write};
use std::iter::Iterator;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::task::Poll;
use std::{fs, io, os, result};

use futures::stream::StreamExt;
use ion::class::{ClassObjectWrapper, Reflector};
use ion::flags::PropertyFlags;
use ion::function::Opt;
use ion::typedarray::Uint8ArrayWrapper;
use ion::{ClassDefinition, Context, Error, Object, Promise, Result};
use mozjs::jsapi::JSFunctionSpec;
use runtime::globals::file::BufferSource;
use runtime::module::NativeModule;
use runtime::promise::future_to_promise;
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::ReadDirStream;
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_FLAGS_AND_ATTRIBUTES};

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

fn open_error(path: &str, err: io::Error) -> String {
	format!("Could not open file: {}\n{}", path, err)
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
			Err(err) => Err(open_error(&path_str, err)),
		}
	})
}

#[js_fn]
fn open_sync(path_str: String, Opt(options): Opt<OpenOptions>) -> Result<ClassObjectWrapper<FileHandle>> {
	let path = Path::new(&path_str);
	let options = options.unwrap_or_default().into_std();

	match options.open(path) {
		Ok(file) => Ok(ClassObjectWrapper(Box::new(FileHandle::new(&path_str, file)))),
		Err(err) => Err(Error::new(open_error(&path_str, err), None)),
	}
}

fn read_file_error(path: &str, err: io::Error) -> String {
	format!("Could not read file: {}\n{}", path, err)
}

fn write_file_error(path: &str, err: io::Error) -> String {
	format!("Could not write to file: {}\n{}", path, err)
}

#[js_class]
pub struct FileHandle {
	reflector: Reflector,
	#[trace(no_trace)]
	path: Arc<str>,
	#[trace(no_trace)]
	handle: Rc<RefCell<Option<File>>>,
}

impl FileHandle {
	fn new(path: &str, file: File) -> FileHandle {
		FileHandle {
			reflector: Reflector::new(),
			path: Arc::from(path),
			handle: Rc::new(RefCell::new(Some(file))),
		}
	}

	fn with_sync<F, T>(&self, callback: F) -> Result<T>
	where
		F: FnOnce(&mut File) -> Result<T>,
	{
		match &mut *self.handle.borrow_mut() {
			Some(handle) => callback(handle),
			None => Err(Error::new("File is busy due to async operation.", None)),
		}
	}

	fn with_blocking_task<F, T>(&self, callback: F) -> impl Future<Output = Result<T>>
	where
		F: FnOnce(&mut File) -> result::Result<T, String> + Send + 'static,
		T: Send + 'static,
	{
		let handle_cell = Rc::clone(&self.handle);

		async move {
			let mut taken = false;
			let mut handle = poll_fn(|wcx| {
				if let Ok(mut handle) = handle_cell.try_borrow_mut() {
					if let Some(file) = handle.as_mut() {
						let file = file.try_clone().ok().unwrap_or_else(|| {
							taken = true;
							handle.take().unwrap()
						});
						return Poll::Ready(file);
					}
				}

				wcx.waker().wake_by_ref();
				Poll::Pending
			})
			.await;

			let (handle, result) = spawn_blocking(move || {
				let result = callback(&mut handle);
				(handle, result)
			})
			.await
			.unwrap();

			if taken {
				handle_cell.borrow_mut().replace(handle);
			}

			result.map_err(|err| Error::new(err, None))
		}
	}
}

#[js_class]
impl FileHandle {
	pub fn read<'cx>(&self, cx: &'cx Context) -> Option<Promise<'cx>> {
		let path = Arc::clone(&self.path);
		future_to_promise(
			cx,
			self.with_blocking_task(move |file| {
				let mut bytes = Vec::new();
				match file.read_to_end(&mut bytes) {
					Ok(_) => Ok(Uint8ArrayWrapper::from(bytes)),
					Err(err) => Err(read_file_error(&path, err)),
				}
			}),
		)
	}

	#[ion(name = "readSync")]
	pub fn read_sync(&self) -> Result<Uint8ArrayWrapper> {
		self.with_sync(|file| {
			let mut bytes = Vec::new();
			match file.read_to_end(&mut bytes) {
				Ok(_) => Ok(Uint8ArrayWrapper::from(bytes)),
				Err(err) => Err(Error::new(read_file_error(&self.path, err), None)),
			}
		})
	}

	pub fn read_string<'cx>(&self, cx: &'cx Context) -> Option<Promise<'cx>> {
		let path = Arc::clone(&self.path);
		future_to_promise(
			cx,
			self.with_blocking_task(move |file| {
				let mut string = String::new();
				match file.read_to_string(&mut string) {
					Ok(_) => Ok(string),
					Err(err) => Err(read_file_error(&path, err)),
				}
			}),
		)
	}

	#[ion(name = "readStringSync")]
	pub fn read_string_sync(&self) -> Result<String> {
		self.with_sync(|file| {
			let mut string = String::new();
			match file.read_to_string(&mut string) {
				Ok(_) => Ok(string),
				Err(err) => Err(Error::new(read_file_error(&self.path, err), None)),
			}
		})
	}

	pub fn write<'cx>(
		&self, cx: &'cx Context, #[ion(convert = false)] contents: BufferSource<'cx>,
	) -> Option<Promise<'cx>> {
		let path = Arc::clone(&self.path);
		let contents = contents.to_vec();
		future_to_promise(
			cx,
			self.with_blocking_task(move |file| match file.write_all(&contents) {
				Ok(_) => Ok(()),
				Err(err) => Err(write_file_error(&path, err)),
			}),
		)
	}

	#[ion(name = "writeSync")]
	pub fn write_sync(&self, #[ion(convert = false)] contents: BufferSource) -> Result<()> {
		self.with_sync(|file| {
			let contents = unsafe { contents.as_slice() };
			match file.write_all(contents) {
				Ok(_) => Ok(()),
				Err(err) => Err(Error::new(write_file_error(&self.path, err), None)),
			}
		})
	}
}

fn read_dir_error(path: &str, err: io::Error) -> Error {
	Error::new(format!("Could not read directory: {}\n{}", path, err), None)
}

fn create_dir_error(path: &str, err: io::Error) -> Error {
	Error::new(format!("Could not create directory: {}\n{}", path, err), None)
}

fn remove_error(path: &str, err: io::Error) -> Error {
	Error::new(format!("Could not remove: {}\n{}", path, err), None)
}

fn copy_error(from: &str, to: &str, err: io::Error) -> Error {
	Error::new(format!("Could not copy from {} to {}\n{}", from, to, err), None)
}

fn rename_error(from: &str, to: &str, err: io::Error) -> Error {
	Error::new(format!("Could not rename from {} to {}\n{}", from, to, err), None)
}

fn symlink_error(original: &str, link: &str, err: io::Error) -> Error {
	Error::new(format!("Could not symlink {} to {}\n{}", original, link, err), None)
}

fn link_error(original: &str, link: &str, err: io::Error) -> Error {
	Error::new(format!("Could not link {} to {}\n{}", original, link, err), None)
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
			Err(err) => Err(create_dir_error(&path_str, err)),
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
		Err(err) => Err(create_dir_error(&path_str, err)),
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

		tokio::fs::copy(from, to).await.map_err(|err| copy_error(&from_str, &to_str, err))
	})
}

#[js_fn]
fn copy_sync(from_str: String, to_str: String) -> Result<u64> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	fs::copy(from, to).map_err(|err| copy_error(&from_str, &to_str, err))
}

#[js_fn]
fn rename(cx: &Context, from_str: String, to_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let from = Path::new(&from_str);
		let to = Path::new(&to_str);

		tokio::fs::rename(from, to).await.map_err(|err| rename_error(&from_str, &to_str, err))
	})
}

#[js_fn]
fn rename_sync(from_str: String, to_str: String) -> Result<()> {
	let from = Path::new(&from_str);
	let to = Path::new(&to_str);

	fs::rename(from, to).map_err(|err| rename_error(&from_str, &to_str, err))
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
		result.map_err(|err| symlink_error(&original_str, &link_str, err))
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
	result.map_err(|err| symlink_error(&original_str, &link_str, err))
}

#[js_fn]
fn link(cx: &Context, original_str: String, link_str: String) -> Option<Promise> {
	future_to_promise(cx, async move {
		let original = Path::new(&original_str);
		let link = Path::new(&link_str);

		tokio::fs::hard_link(original, link)
			.await
			.map_err(|err| link_error(&original_str, &link_str, err))
	})
}

#[js_fn]
fn link_sync(original_str: String, link_str: String) -> Result<()> {
	let original = Path::new(&original_str);
	let link = Path::new(&link_str);

	fs::hard_link(original, link).map_err(|err| link_error(&original_str, &link_str, err))
}

const SYNC_FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(open_sync, "open", 1),
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
