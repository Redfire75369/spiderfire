/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use mozjs::jsapi::CallArgs;
use mozjs::jsval::JSVal;

use crate::{Context, Local, Value};

/// Represents Arguments to a [JavaScript Function](crate::Function)
/// Wrapper around [CallArgs] to provide lifetimes and root all arguments.
#[derive(Debug)]
pub struct Arguments<'cx> {
	values: Vec<Value<'cx>>,
	this: Value<'cx>,
	rval: Value<'cx>,
	call_args: CallArgs,
}

impl<'cx> Arguments<'cx> {
	/// Creates new [Arguments] from raw arguments,
	pub fn new(cx: &'cx Context, argc: u32, vp: *mut JSVal) -> Arguments<'cx> {
		unsafe {
			let call_args = CallArgs::from_vp(vp, argc);
			let values = (0..argc).map(|i| cx.root_value(call_args.get(i).get()).into()).collect();
			let this = cx.root_value(call_args.thisv().get()).into();
			let rval = Local::from_raw_handle_mut(call_args.rval()).into();

			Arguments { values, this, rval, call_args }
		}
	}

	/// Returns the number of arguments passed to the function.
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self) -> usize {
		self.values.len()
	}

	/// Gets the [Value] at the given index.
	/// Returns [None] if the given index is larger than the number of arguments.
	pub fn value(&self, index: usize) -> Option<&Value<'cx>> {
		if index < self.len() {
			return Some(&self.values[index]);
		}
		None
	}

	/// Returns a vector of arguments as [Values](Value) based on the indices of the iterator.
	pub fn range<'a, R: Iterator<Item = usize>>(&'a self, range: R) -> Vec<&'a Value<'cx>>
	where
		'cx: 'a,
	{
		range.filter_map(|index| self.value(index)).collect()
	}

	/// Returns the `this` value of the function.
	/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/this) for more details.
	pub fn this(&mut self) -> &mut Value<'cx> {
		&mut self.this
	}

	/// Returns the return value of the function.
	/// This value can be modified to change the return value.
	pub fn rval(&mut self) -> &mut Value<'cx> {
		&mut self.rval
	}

	/// Returns true if the function was called with `new`.
	pub fn is_constructing(&self) -> bool {
		self.call_args.constructing_()
	}

	/// Returns the raw [CallArgs].
	pub fn call_args(&self) -> CallArgs {
		self.call_args
	}
}
