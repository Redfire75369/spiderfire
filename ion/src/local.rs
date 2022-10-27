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

pub enum Local<'cx, T: 'cx>
where
	T: GCMethods + RootKind,
{
	Rooted(&'cx mut RootedGuard<'cx, T>),
	Mutable(MutableHandle<'cx, T>),
	Handle(Handle<'cx, T>),
}

impl<'cx, T: Copy + GCMethods + RootKind> Local<'cx, T> {
	pub fn handle<'a>(&'a self) -> Handle<'a, T>
	where
		'cx: 'a,
	{
		match self {
			Self::Rooted(rooted) => rooted.handle(),
			Self::Mutable(handle) => handle.handle(),
			Self::Handle(handle) => *handle,
		}
	}

	pub fn handle_mut<'a>(&'a mut self) -> MutableHandle<'a, T>
	where
		'cx: 'a,
	{
		match self {
			Local::Rooted(rooted) => rooted.handle_mut(),
			Local::Mutable(handle) => *handle,
			Local::Handle(_) => panic!("&mut Local::Handle should never be constructed"),
		}
	}
}

impl<'cx, T: GCMethods + RootKind> Local<'cx, T> {
	pub fn from_rooted(rooted: &'cx mut RootedGuard<'cx, T>) -> Local<'cx, T> {
		Local::Rooted(rooted)
	}

	pub fn from_handle_mut(handle: MutableHandle<'cx, T>) -> Local<'cx, T> {
		Local::Mutable(handle)
	}

	pub fn from_handle(handle: Handle<'cx, T>) -> Local<'cx, T> {
		Local::Handle(handle)
	}

	pub unsafe fn from_marked(ptr: *const T) -> Local<'cx, T> {
		Local::Handle(Handle::from_marked_location(ptr))
	}

	pub unsafe fn from_raw_handle(handle: RawHandle<T>) -> Local<'cx, T> {
		Local::Handle(Handle::from_raw(handle))
	}

	pub unsafe fn from_marked_mut(ptr: *mut T) -> Local<'cx, T> {
		Local::Mutable(MutableHandle::from_marked_location(ptr))
	}

	pub unsafe fn from_raw_handle_mut(handle: RawMutableHandle<T>) -> Local<'cx, T> {
		Local::Mutable(MutableHandle::from_raw(handle))
	}
}

impl<'cx, T: GCMethods + RootKind> Deref for Local<'cx, T> {
	type Target = T;

	fn deref(&self) -> &T {
		match self {
			Local::Rooted(rooted) => &***rooted,
			Local::Mutable(handle) => &**handle,
			Local::Handle(handle) => &**handle,
		}
	}
}

impl<'cx, T: GCMethods + RootKind> DerefMut for Local<'cx, T> {
	fn deref_mut(&mut self) -> &mut T {
		match self {
			Local::Rooted(rooted) => &mut ***rooted,
			Local::Mutable(handle) => &mut **handle,
			Local::Handle(_) => panic!("&mut Local::Handle should never be constructed"),
		}
	}
}

impl<'cx, T: Debug + GCMethods + RootKind> Debug for Local<'cx, T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		(**self).fmt(f)
	}
}
