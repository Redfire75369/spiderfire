/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::fs::File;
use std::future::{poll_fn, Future};
use std::io::{Read, Write};
use std::rc::Rc;
use std::sync::Arc;
use std::task::Poll;
use std::{io, result};

use ion::class::Reflector;
use ion::conversions::IntoValue;
use ion::typedarray::Uint8ArrayWrapper;
use ion::{Context, Error, Promise, Result};
use runtime::globals::file::BufferSource;
use runtime::promise::future_to_promise;
use tokio::task::spawn_blocking;

use crate::fs::file_error;

#[js_class]
pub struct FileHandle {
	reflector: Reflector,
	#[trace(no_trace)]
	path: Arc<str>,
	#[trace(no_trace)]
	handle: Rc<RefCell<Option<File>>>,
}

impl FileHandle {
	pub(crate) fn new(path: &str, file: File) -> FileHandle {
		FileHandle {
			reflector: Reflector::new(),
			path: Arc::from(path),
			handle: Rc::new(RefCell::new(Some(file))),
		}
	}

	pub(crate) fn with_sync<F, T>(&self, callback: F) -> Result<T>
	where
		F: FnOnce(&mut File) -> Result<T>,
	{
		match &mut *self.handle.borrow_mut() {
			Some(handle) => callback(handle),
			None => Err(Error::new("File is busy due to async operation.", None)),
		}
	}

	pub(crate) fn with_blocking_task<F, T>(&self, callback: F) -> impl Future<Output = io::Result<T>>
	where
		F: FnOnce(&mut File) -> io::Result<T> + Send + 'static,
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

			result
		}
	}

	pub(crate) fn with_blocking_promise<'cx, F, T>(
		&self, cx: &'cx Context, action: &'static str, path: Arc<str>, callback: F,
	) -> Option<Promise<'cx>>
	where
		F: FnOnce(&mut File) -> result::Result<T, io::Error> + Send + 'static,
		T: for<'cx2> IntoValue<'cx2> + Send + 'static,
	{
		let task = self.with_blocking_task(callback);
		future_to_promise(
			cx,
			async move { task.await.map_err(|err| file_error(action, &path, err)) },
		)
	}
}

#[js_class]
impl FileHandle {
	pub fn read<'cx>(&self, cx: &'cx Context) -> Option<Promise<'cx>> {
		let path = Arc::clone(&self.path);
		self.with_blocking_promise(cx, "read", path, move |file| {
			let mut bytes = Vec::new();
			file.read_to_end(&mut bytes).map(|_| Uint8ArrayWrapper::from(bytes))
		})
	}

	#[ion(name = "readSync")]
	pub fn read_sync(&self) -> Result<Uint8ArrayWrapper> {
		self.with_sync(|file| {
			let mut bytes = Vec::new();
			match file.read_to_end(&mut bytes) {
				Ok(_) => Ok(Uint8ArrayWrapper::from(bytes)),
				Err(err) => Err(file_error("read", &self.path, err)),
			}
		})
	}

	pub fn read_string<'cx>(&self, cx: &'cx Context) -> Option<Promise<'cx>> {
		let path = Arc::clone(&self.path);
		self.with_blocking_promise(cx, "read", path, move |file| {
			let mut string = String::new();
			file.read_to_string(&mut string).map(|_| string)
		})
	}

	#[ion(name = "readStringSync")]
	pub fn read_string_sync(&self) -> Result<String> {
		self.with_sync(|file| {
			let mut string = String::new();
			match file.read_to_string(&mut string) {
				Ok(_) => Ok(string),
				Err(err) => Err(file_error("read", &self.path, err)),
			}
		})
	}

	pub fn write<'cx>(
		&self, cx: &'cx Context, #[ion(convert = false)] contents: BufferSource<'cx>,
	) -> Option<Promise<'cx>> {
		let path = Arc::clone(&self.path);
		let contents = contents.to_vec();
		self.with_blocking_promise(cx, "write", path, move |file| file.write_all(&contents).map(|_| ()))
	}

	#[ion(name = "writeSync")]
	pub fn write_sync(&self, #[ion(convert = false)] contents: BufferSource) -> Result<()> {
		self.with_sync(|file| {
			let contents = unsafe { contents.as_slice() };
			match file.write_all(contents) {
				Ok(_) => Ok(()),
				Err(err) => Err(file_error("write", &self.path, err)),
			}
		})
	}
}
