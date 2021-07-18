/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::ops::RangeBounds;

use mozjs::jsapi::{CallArgs, Handle, MutableHandle, UndefinedHandleValue, Value};
use mozjs::jsval::UndefinedValue;

pub struct Arguments {
	pub values: Vec<Handle<Value>>,
	pub this: Handle<Value>,
	pub rval: MutableHandle<Value>,
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

	pub fn len(&self) -> usize {
		self.values.len()
	}

	#[allow(dead_code)]
	pub fn handle(&self, index: usize) -> Option<Handle<Value>> {
		if self.len() > index + 1 {
			return Some(self.values[index]);
		}
		None
	}

	pub fn handle_or_undefined(&self, index: usize) -> Handle<Value> {
		if self.len() > index + 1 {
			return self.values[index];
		}
		unsafe { UndefinedHandleValue }
	}

	#[allow(dead_code)]
	pub fn value(&self, index: usize) -> Option<Value> {
		if self.len() > index + 1 {
			return Some(self.values[index].get());
		}
		None
	}

	#[allow(dead_code)]
	pub fn value_or_undefined(&self, index: usize) -> Value {
		if self.len() > index + 1 {
			return self.values[index].get();
		}
		UndefinedValue()
	}

	#[allow(dead_code)]
	pub fn range<R: Iterator<Item = usize> + RangeBounds<usize>>(&self, range: R) -> Vec<Value> {
		range.filter_map(|index| self.value(index)).collect::<Vec<_>>()
	}

	pub fn range_handles<R: Iterator<Item = usize> + RangeBounds<usize>>(&self, range: R) -> Vec<Handle<Value>> {
		range.filter_map(|index| self.handle(index)).collect::<Vec<_>>()
	}

	#[allow(dead_code)]
	pub fn range_full(&self) -> Vec<Value> {
		self.values.iter().map(|value| value.get()).collect::<Vec<_>>()
	}
}
