/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};
use std::path::{Component, Path, PathBuf};

use mozjs::conversions::ToJSValConvertible;
use mozjs::jsval::ObjectOrNullValue;
use mozjs::rust::MutableHandleValue;
use mozjs::typedarray::{CreateWith, Uint8Array};

use crate::{Context, Error, Object};
use crate::error::ThrowException;

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

pub struct Uint8ArrayBuffer {
	pub buf: Vec<u8>,
}

impl Deref for Uint8ArrayBuffer {
	type Target = Vec<u8>;

	fn deref(&self) -> &Self::Target {
		&self.buf
	}
}

impl DerefMut for Uint8ArrayBuffer {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.buf
	}
}

impl ToJSValConvertible for Uint8ArrayBuffer {
	unsafe fn to_jsval(&self, cx: Context, mut rval: MutableHandleValue) {
		rooted!(in(cx) let mut array = *Object::new(cx));
		if Uint8Array::create(cx, CreateWith::Slice(self.buf.as_slice()), array.handle_mut()).is_ok() {
			rval.set(ObjectOrNullValue(array.get()));
		} else {
			Error::new("Failed to create Uint8Array", None).throw(cx)
		}
	}
}
