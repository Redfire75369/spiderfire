/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::mem::MaybeUninit;
use std::path::{Component, Path, PathBuf};
use std::slice;
use std::slice::Iter;

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
		(Box::into_raw(self).cast::<T>(), len)
	}

	unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> Self {
		unsafe {
			let slice = slice::from_raw_parts_mut(ptr, len);
			Box::from_raw(slice)
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub struct ArrayVec<const CAP: usize, T: Copy> {
	elements: [MaybeUninit<T>; CAP],
	len: usize,
}

impl<const CAP: usize, T: Copy> ArrayVec<CAP, T> {
	pub const fn new() -> ArrayVec<CAP, T> {
		ArrayVec {
			elements: unsafe { MaybeUninit::uninit().assume_init() },
			len: 0,
		}
	}

	pub const fn len(&self) -> usize {
		self.len
	}

	pub const fn is_empty(&self) -> bool {
		self.len() == 0
	}

	pub const fn is_full(&self) -> bool {
		self.len() == CAP
	}

	pub const fn push(mut self, element: T) -> ArrayVec<CAP, T> {
		if self.len == CAP {
			panic!("Exceeded capacity of ArrayVec.");
		}
		self.elements[self.len] = MaybeUninit::new(element);
		self.len += 1;
		self
	}

	pub const fn pop(mut self) -> (ArrayVec<CAP, T>, Option<T>) {
		if self.len == 0 {
			return (self, None);
		}
		let element = unsafe { self.elements[self.len].assume_init() };
		self.len -= 1;
		(self, Some(element))
	}

	pub const fn get(&self, index: usize) -> Option<&T> {
		if self.is_empty() || index >= self.len() {
			return None;
		}
		Some(unsafe { self.elements[index].assume_init_ref() })
	}

	pub const fn truncate(mut self, new_len: usize) -> ArrayVec<CAP, T> {
		if new_len >= self.len {
			return self;
		}
		self.len = new_len;
		self
	}

	pub fn iter(&self) -> Iter<'_, T> {
		unsafe { slice::from_raw_parts(self.elements.as_ptr().cast::<T>(), self.len()).iter() }
	}
}
