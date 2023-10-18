/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Debug, Formatter};

use mozjs::gc::{GCMethods, Handle, MutableHandle};
use mozjs::jsapi::{Heap, Rooted};
use mozjs::jsapi::Handle as RawHandle;
use mozjs::jsapi::MutableHandle as RawMutableHandle;
use mozjs_sys::jsgc::RootKind;

use crate::Context;

/// Represents a local reference managed by the Garbage Collector.
/// Prevents a local value that is currently being used from being garbage collected and causing undefined behaviour.
pub enum Local<'local, T: 'local>
where
	T: GCMethods + RootKind,
{
	Rooted(&'local mut Rooted<T>),
	Mutable(MutableHandle<'local, T>),
	Handle(Handle<'local, T>),
}

impl<'local, T: Copy + GCMethods + RootKind> Local<'local, T> {
	/// Forms a [Handle] to the [Local] which can be passed to SpiderMonkey APIs.
	pub fn handle<'a>(&'a self) -> Handle<'a, T>
	where
		'local: 'a,
	{
		match self {
			Self::Rooted(root) => unsafe { Handle::from_marked_location(&root.ptr) },
			Self::Mutable(handle) => handle.handle(),
			Self::Handle(handle) => *handle,
		}
	}

	/// Forms a [Handle] to the [Local] which can be passed to SpiderMonkey APIs.
	///
	/// ### Panics
	/// Panics when a [`Local::Handle`] is passed.
	pub fn handle_mut<'a>(&'a mut self) -> MutableHandle<'a, T>
	where
		'local: 'a,
	{
		match self {
			Local::Rooted(root) => unsafe { MutableHandle::from_marked_location(&mut root.ptr) },
			Local::Mutable(handle) => *handle,
			Local::Handle(_) => panic!("&mut Local::Handle should never be constructed"),
		}
	}

	pub fn get(&self) -> T {
		match self {
			Self::Rooted(root) => root.ptr,
			Self::Mutable(handle) => handle.get(),
			Self::Handle(handle) => handle.get(),
		}
	}
}

impl<'local, T: GCMethods + RootKind> Local<'local, T> {
	/// Creates a new [Local].
	/// `Context::root_*` should be used instead.
	pub(crate) fn new(cx: &Context, root: &'local mut Rooted<T>, initial: T) -> Local<'local, T> {
		root.ptr = initial;
		unsafe {
			root.add_to_root_stack(cx.as_ptr());
		}
		Local::Rooted(root)
	}

	/// Creates a [Local] from a [MutableHandle].
	pub fn from_handle_mut(handle: MutableHandle<'local, T>) -> Local<'local, T> {
		Local::Mutable(handle)
	}

	/// Creates a [Local] from a [Handle].
	pub fn from_handle(handle: Handle<'local, T>) -> Local<'local, T> {
		Local::Handle(handle)
	}

	/// Creates a [Local] from a marked (pointer).
	///
	/// ### Safety
	/// The pointer must point to a location marked by the Garbage Collector.
	pub unsafe fn from_marked(ptr: *const T) -> Local<'local, T> {
		Local::Handle(unsafe { Handle::from_marked_location(ptr) })
	}

	/// Creates a [Local] from a [raw handle](RawHandle).
	///
	/// ### Safety
	/// The handle must be valid and still be recognised by the Garbage Collector.
	pub unsafe fn from_raw_handle(handle: RawHandle<T>) -> Local<'local, T> {
		Local::Handle(unsafe { Handle::from_raw(handle) })
	}

	/// Creates a [Local] from a marked [mutable pointer](pointer);
	///
	/// ### Safety
	/// The pointer must point to a location marked by the Garbage Collector.
	pub unsafe fn from_marked_mut(ptr: *mut T) -> Local<'local, T> {
		Local::Mutable(unsafe { MutableHandle::from_marked_location(ptr) })
	}

	/// Creates a [Local] from a [raw mutable handle](RawMutableHandle);
	///
	/// ### Safety
	/// The mutable handle must be valid and still be recognised by the Garbage Collector.
	pub unsafe fn from_raw_handle_mut(handle: RawMutableHandle<T>) -> Local<'local, T> {
		Local::Mutable(unsafe { MutableHandle::from_raw(handle) })
	}
}

impl<'local, T: Copy + GCMethods + RootKind> Local<'local, T> {
	pub unsafe fn from_heap(heap: &Heap<T>) -> Local<'local, T> {
		unsafe { Local::from_raw_handle(heap.handle()) }
	}
}

impl<T: Copy + Debug + GCMethods + RootKind> Debug for Local<'_, T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		self.handle().fmt(f)
	}
}
