/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::{Component, Path, PathBuf};
use std::slice;

/// Normalises a [Path] by removing all `./` and resolving all `../` simplistically.
/// This function does not follow symlinks and may result in unexpected behaviour.
pub fn normalise_path<P: AsRef<Path>>(path: P) -> PathBuf {
	let mut buf = PathBuf::new();
	let segments = path.as_ref().components();

	for segment in segments {
		match segment {
			Component::ParentDir => {
				let len = buf.components().count();
				if len == 0 || buf.components().all(|c| matches!(c, Component::ParentDir)) {
					buf.push("..");
				} else {
					buf.pop();
				}
			}
			Component::CurDir => {}
			segment => buf.push(segment),
		}
	}
	buf
}

pub trait BoxExt<T> {
	unsafe fn into_raw_parts(self) -> (*mut T, usize);

	unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> Self;
}

impl<T> BoxExt<T> for Box<[T]> {
	unsafe fn into_raw_parts(self) -> (*mut T, usize) {
		let len = self.len();
		(Box::into_raw(self).cast(), len)
	}

	unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> Self {
		unsafe {
			let slice = slice::from_raw_parts_mut(ptr, len);
			Box::from_raw(slice)
		}
	}
}
