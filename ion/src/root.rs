/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::UnsafeCell;
use std::fmt;
use std::fmt::{Debug, Formatter};

use mozjs::gc::{GCMethods, Traceable};
use mozjs::jsapi::Heap;
use mozjs::rust::Handle;

use crate::context::{RootCollection, StableTraceable};

pub struct Root<T: StableTraceable>
where
	Heap<T::Traced>: Traceable,
{
	value: T,
	roots: *const RootCollection,
}

impl<T: StableTraceable> Root<T>
where
	Heap<T::Traced>: Traceable,
{
	pub(crate) unsafe fn new(value: T, roots: *const RootCollection) -> Root<T> {
		Root { value, roots }
	}

	pub fn handle(&self) -> Handle<T::Traced> {
		unsafe { Handle::from_raw(self.value.heap().handle()) }
	}
}

impl<T: StableTraceable> Root<T>
where
	Heap<T::Traced>: Traceable,
{
	pub fn get(&self) -> T::Traced {
		self.handle().get()
	}
}

impl<T: Copy + GCMethods + 'static> Clone for Root<Box<Heap<T>>>
where
	Heap<T>: Traceable,
{
	fn clone(&self) -> Root<Box<Heap<T>>> {
		let heap = Box::new(Heap {
			ptr: UnsafeCell::new(unsafe { T::initial() }),
		});
		heap.set(self.get());
		unsafe {
			(*self.roots).root(heap.heap());
			Root::new(heap, self.roots)
		}
	}

	fn clone_from(&mut self, source: &Root<Box<Heap<T>>>) {
		self.value.set(source.get());
		self.roots = source.roots;
	}
}

impl<T: StableTraceable> Debug for Root<T>
where
	T::Traced: Debug,
	Heap<T::Traced>: Traceable,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("Root").field("value", unsafe { &*self.value.heap().get_unsafe() }).finish()
	}
}

impl<T: StableTraceable> Drop for Root<T>
where
	Heap<T::Traced>: Traceable,
{
	fn drop(&mut self) {
		if !self.roots.is_null() {
			unsafe {
				(*self.roots).unroot(self.value.heap());
			}
		}
	}
}
