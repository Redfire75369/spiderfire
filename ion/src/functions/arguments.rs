/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::ops::RangeBounds;

use mozjs::jsapi::*;

pub struct Arguments {
	pub values: Vec<Handle<Value>>,
	call_args: CallArgs,
}

impl Arguments {
	pub unsafe fn new(argc: u32, vp: *mut Value) -> Arguments {
		let call_args = CallArgs::from_vp(vp, argc);

		let values: Vec<_> = (0..(argc + 1)).map(|i| call_args.get(i)).collect();

		Arguments { values, call_args }
	}

	pub fn len(&self) -> usize {
		self.values.len()
	}

	pub fn handle(&self, index: usize) -> Option<Handle<Value>> {
		if self.len() > index + 1 {
			return Some(self.values[index]);
		}
		None
	}

	pub fn value(&self, index: usize) -> Option<Value> {
		if self.len() > index + 1 {
			return Some(self.values[index].get());
		}
		None
	}

	pub fn range<R: Iterator<Item = usize> + RangeBounds<usize>>(&self, range: R) -> Vec<Value> {
		range.filter_map(|index| self.value(index)).collect::<Vec<_>>()
	}

	pub fn range_handles<R: Iterator<Item = usize> + RangeBounds<usize>>(&self, range: R) -> Vec<Handle<Value>> {
		range.filter_map(|index| self.handle(index)).collect::<Vec<_>>()
	}

	pub fn range_full(&self) -> Vec<Value> {
		self.values.iter().map(|value| value.get()).collect::<Vec<_>>()
	}

	pub fn thisv(&self) -> Handle<Value> {
		self.call_args.thisv()
	}

	pub fn rval(&self) -> MutableHandle<Value> {
		self.call_args.rval()
	}
}
