/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::UnsafeCell;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ptr::NonNull;

use mozjs::gc::GCMethods;
use mozjs::jsapi::Heap;
use mozjs::rust::Handle;

use crate::context::{RootCollection, StableTraceable};

pub struct Root<T: StableTraceable> {
	value: T,
	roots: NonNull<RootCollection>,
}

impl<T: StableTraceable> Root<T> {
	pub(crate) unsafe fn new(value: T, roots: NonNull<RootCollection>) -> Root<T> {
		Root { value, roots }
	}

	pub fn handle(&self) -> Handle<T::Traced> {
		unsafe { Handle::from_marked_location(self.value.traced()) }
	}
}

impl<T: StableTraceable> Root<T>
where
	T::Traced: Copy,
{
	pub fn get(&self) -> T::Traced {
		self.handle().get()
	}
}

impl<T: Copy + GCMethods> Clone for Root<Box<Heap<T>>>
where
	Box<Heap<T>>: StableTraceable<Traced = T>,
{
	fn clone(&self) -> Root<Box<Heap<T>>> {
		let heap = Box::new(Heap {
			ptr: UnsafeCell::new(unsafe { T::initial() }),
		});
		heap.set(self.get());
		unsafe {
			(*self.roots.as_ptr()).root(heap.traceable());
			Root::new(heap, self.roots)
		}
	}

	fn clone_from(&mut self, source: &Root<Box<Heap<T>>>) {
		self.value.set(source.get());
		self.roots = source.roots;
	}
}

impl<T: Debug + StableTraceable> Debug for Root<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("Root").field("value", &self.value).finish()
	}
}

impl<T: StableTraceable + 'static> Drop for Root<T> {
	fn drop(&mut self) {
		unsafe {
			(*self.roots.as_ptr()).unroot(self.value.traceable());
		}
	}
}
