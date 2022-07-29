/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::RangeBounds;

use mozjs::jsapi::{CallArgs, Handle, MutableHandle, UndefinedHandleValue};
use mozjs::jsval::{JSVal, UndefinedValue};

/// Function Arguments
#[derive(Clone, Debug)]
pub struct Arguments {
	values: Vec<Handle<JSVal>>,
	this: Handle<JSVal>,
	rval: MutableHandle<JSVal>,
	#[allow(dead_code)]
	call_args: CallArgs,
}

impl Arguments {
	/// Creates new [Arguments] from raw argument,
	pub fn new(argc: u32, vp: *mut JSVal) -> Arguments {
		let call_args = unsafe { CallArgs::from_vp(vp, argc) };
		let values = (0..argc).map(|i| call_args.get(i)).collect();
		let this = call_args.thisv();
		let rval = call_args.rval();

		Arguments { values, this, rval, call_args }
	}

	/// Returns the number of arguments passed to the function.
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self) -> usize {
		self.values.len()
	}

	/// Gets the handle of the value at the given index.
	/// Returns [None] if the given index is larger than the number of arguments.
	pub fn handle(&self, index: usize) -> Option<Handle<JSVal>> {
		if index < self.len() {
			return Some(self.values[index]);
		}
		None
	}

	/// Gets the handle of the value at the given index.
	/// Returns `undefined` if the given index is larger than the number of arguments.
	pub fn handle_or_undefined(&self, index: usize) -> Handle<JSVal> {
		if index < self.len() {
			return self.values[index];
		}
		unsafe { UndefinedHandleValue }
	}

	/// Gets the value at the given index.
	/// Returns [None] if the given index is larger than the number of arguments.
	pub fn value(&self, index: usize) -> Option<JSVal> {
		if index < self.len() {
			return Some(self.values[index].get());
		}
		None
	}

	/// Gets the value at the given index.
	/// Returns `undefined` if the given index is larger than the number of arguments.
	pub fn value_or_undefined(&self, index: usize) -> JSVal {
		if index < self.len() {
			return self.values[index].get();
		}
		UndefinedValue()
	}

	/// Returns a range of values within the arguments.
	pub fn range<R: Iterator<Item = usize> + RangeBounds<usize>>(&self, range: R) -> Vec<JSVal> {
		range.filter_map(|index| self.value(index)).collect()
	}

	/// Returns a range of handles within the arguments.
	pub fn range_handles<R: Iterator<Item = usize> + RangeBounds<usize>>(&self, range: R) -> Vec<Handle<JSVal>> {
		range.filter_map(|index| self.handle(index)).collect()
	}

	/// Returns a [Vec] with all arguments
	pub fn range_full(&self) -> Vec<JSVal> {
		self.values.iter().map(|value| value.get()).collect()
	}

	/// Returns the `this` value of the function.
	pub fn this(&self) -> Handle<JSVal> {
		self.this
	}

	/// Returns the mutable return value of the function.
	pub fn rval(&self) -> MutableHandle<JSVal> {
		self.rval
	}

	/// Returns true if the function was called with `new`,
	pub fn is_constructing(&self) -> bool {
		self.call_args.constructing_()
	}

	pub fn call_args(&self) -> CallArgs {
		self.call_args
	}
}
