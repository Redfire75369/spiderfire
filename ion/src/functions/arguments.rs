/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::{CallArgs, Handle, MutableHandle, UndefinedHandleValue, Value};
use mozjs::jsval::UndefinedValue;
use std::ops::RangeBounds;

#[derive(Clone, Debug)]
pub struct Arguments {
	values: Vec<Handle<Value>>,
	this: Handle<Value>,
	rval: MutableHandle<Value>,
	#[allow(dead_code)]
	call_args: CallArgs,
}

impl Arguments {
	pub unsafe fn new(argc: u32, vp: *mut Value) -> Arguments {
		let call_args = CallArgs::from_vp(vp, argc);
		let values: Vec<_> = (0..(argc + 1)).map(|i| call_args.get(i)).collect();
		let this = call_args.thisv();
		let rval = call_args.rval();

		Arguments {
			values,
			this,
			rval,
			call_args,
		}
	}

	/// Returns the number of arguments.
	pub fn len(&self) -> usize {
		self.values.len()
	}

	/// Gets the handle of the value at the given index.
	///
	/// Returns [None] if the given index is larger than the number of arguments.
	pub fn handle(&self, index: usize) -> Option<Handle<Value>> {
		if self.len() > index + 1 {
			return Some(self.values[index]);
		}
		None
	}

	/// Gets the handle of the value at the given index.
	///
	/// Returns `undefined` if the given index is larger than the number of arguments.
	pub fn handle_or_undefined(&self, index: usize) -> Handle<Value> {
		if self.len() > index + 1 {
			return self.values[index];
		}
		unsafe { UndefinedHandleValue }
	}

	/// Gets the value at the given index.
	///
	/// Returns [None] if the given index is larger than the number of arguments.
	pub fn value(&self, index: usize) -> Option<Value> {
		if self.len() > index + 1 {
			return Some(self.values[index].get());
		}
		None
	}

	/// Gets the value at the given index.
	///
	/// Returns `undefined` if the given index is larger than the number of arguments.
	pub fn value_or_undefined(&self, index: usize) -> Value {
		if self.len() > index + 1 {
			return self.values[index].get();
		}
		UndefinedValue()
	}

	pub fn range<R: Iterator<Item = usize> + RangeBounds<usize>>(&self, range: R) -> Vec<Value> {
		range.filter_map(|index| self.value(index)).collect()
	}

	pub fn range_handles<R: Iterator<Item = usize> + RangeBounds<usize>>(&self, range: R) -> Vec<Handle<Value>> {
		range.filter_map(|index| self.handle(index)).collect()
	}

	pub fn range_full(&self) -> Vec<Value> {
		self.values.iter().map(|value| value.get()).collect()
	}

	/// Returns the `this` value of the current scope.
	pub fn this(&self) -> Handle<Value> {
		self.this
	}

	/// Returns the mutable return value of the function.
	pub fn rval(&self) -> MutableHandle<Value> {
		self.rval
	}
}
