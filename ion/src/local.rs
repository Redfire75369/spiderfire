/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

use mozjs::jsapi::Handle as RawHandle;
use mozjs::jsapi::MutableHandle as RawMutableHandle;
use mozjs::rust::{GCMethods, Handle, MutableHandle, RootedGuard};
use mozjs_sys::jsgc::RootKind;

pub enum Local<'local, T: 'local>
where
	T: GCMethods + RootKind,
{
	Rooted(&'local mut RootedGuard<'local, T>),
	Mutable(MutableHandle<'local, T>),
	Handle(Handle<'local, T>),
}

impl<'local, T: Copy + GCMethods + RootKind> Local<'local, T> {
	pub fn handle<'a>(&'a self) -> Handle<'a, T>
	where
		'local: 'a,
	{
		match self {
			Self::Rooted(rooted) => rooted.handle(),
			Self::Mutable(handle) => handle.handle(),
			Self::Handle(handle) => *handle,
		}
	}

	pub fn handle_mut<'a>(&'a mut self) -> MutableHandle<'a, T>
	where
		'local: 'a,
	{
		match self {
			Local::Rooted(rooted) => rooted.handle_mut(),
			Local::Mutable(handle) => *handle,
			Local::Handle(_) => panic!("&mut Local::Handle should never be constructed"),
		}
	}
}

impl<'local, T: GCMethods + RootKind> Local<'local, T> {
	pub fn from_rooted(rooted: &'local mut RootedGuard<'local, T>) -> Local<'local, T> {
		Local::Rooted(rooted)
	}

	pub fn from_handle_mut(handle: MutableHandle<'local, T>) -> Local<'local, T> {
		Local::Mutable(handle)
	}

	pub fn from_handle(handle: Handle<'local, T>) -> Local<'local, T> {
		Local::Handle(handle)
	}

	pub unsafe fn from_marked(ptr: *const T) -> Local<'local, T> {
		Local::Handle(Handle::from_marked_location(ptr))
	}

	pub unsafe fn from_raw_handle(handle: RawHandle<T>) -> Local<'local, T> {
		Local::Handle(Handle::from_raw(handle))
	}

	pub unsafe fn from_marked_mut(ptr: *mut T) -> Local<'local, T> {
		Local::Mutable(MutableHandle::from_marked_location(ptr))
	}

	pub unsafe fn from_raw_handle_mut(handle: RawMutableHandle<T>) -> Local<'local, T> {
		Local::Mutable(MutableHandle::from_raw(handle))
	}
}

impl<T: GCMethods + RootKind> Deref for Local<'_, T> {
	type Target = T;

	fn deref(&self) -> &T {
		match self {
			Local::Rooted(rooted) => &***rooted,
			Local::Mutable(handle) => &**handle,
			Local::Handle(handle) => &**handle,
		}
	}
}

impl<T: GCMethods + RootKind> DerefMut for Local<'_, T> {
	fn deref_mut(&mut self) -> &mut T {
		match self {
			Local::Rooted(rooted) => &mut ***rooted,
			Local::Mutable(handle) => &mut **handle,
			Local::Handle(_) => panic!("&mut Local::Handle should never be constructed"),
		}
	}
}

impl<T: Debug + GCMethods + RootKind> Debug for Local<'_, T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		(**self).fmt(f)
	}
}
