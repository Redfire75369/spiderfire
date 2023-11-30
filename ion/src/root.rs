/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ptr::NonNull;

use mozjs::rust::Handle;

use crate::context::{RootCollection, StableTraceable};

pub struct Root<T: StableTraceable> {
	value: T,
	roots: Option<NonNull<RootCollection>>,
}

impl<T: StableTraceable> Root<T> {
	pub(crate) unsafe fn new(value: T, roots: Option<NonNull<RootCollection>>) -> Root<T> {
		Root { value, roots }
	}

	pub fn handle(&self) -> Handle<T::Trace> {
		unsafe { Handle::from_marked_location(self.value.stable_trace()) }
	}
}

impl<T: StableTraceable> Root<T>
where
	T::Trace: Copy,
{
	pub fn get(&self) -> T::Trace {
		self.handle().get()
	}
}

impl<T: Debug + StableTraceable> Debug for Root<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("Root").field("value", &self.value).finish()
	}
}

impl<T: StableTraceable + 'static> Drop for Root<T> {
	fn drop(&mut self) {
		if let Some(roots) = self.roots {
			unsafe {
				(*roots.as_ptr()).unroot(self.value.traceable());
			}
		}
	}
}
