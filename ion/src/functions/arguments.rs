/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use mozjs::jsapi::CallArgs;
use mozjs::jsval::JSVal;

use crate::{Context, Local, Object, Result, Value};
use crate::conversions::FromValue;

/// Represents Arguments to a [JavaScript Function](crate::Function)
/// Wrapper around [CallArgs] to provide lifetimes and root all arguments.
pub struct Arguments<'cx> {
	cx: &'cx Context,
	values: Box<[Value<'cx>]>,
	callee: Object<'cx>,
	this: Value<'cx>,
	rval: Value<'cx>,
	call_args: CallArgs,
}

impl<'cx> Arguments<'cx> {
	/// Creates new [Arguments] from raw arguments,
	pub unsafe fn new(cx: &'cx Context, argc: u32, vp: *mut JSVal) -> Arguments<'cx> {
		unsafe {
			let call_args = CallArgs::from_vp(vp, argc);
			let values = (0..argc).map(|i| Local::from_raw_handle(call_args.get(i)).into()).collect();
			let callee = cx.root_object(call_args.callee()).into();
			let this = cx.root_value(call_args.thisv().get()).into();
			let rval = Local::from_raw_handle_mut(call_args.rval()).into();

			Arguments {
				cx,
				values,
				callee,
				this,
				rval,
				call_args,
			}
		}
	}

	/// Returns the number of arguments passed to the function.
	pub fn len(&self) -> usize {
		self.values.len()
	}

	pub fn is_empty(&self) -> bool {
		self.values.len() == 0
	}

	pub fn access(&mut self) -> Accessor<'_, 'cx> {
		Accessor { args: self, index: 0 }
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
	pub fn range<R: Iterator<Item = usize>>(&self, range: R) -> Vec<&Value<'cx>> {
		range.filter_map(|index| self.value(index)).collect()
	}

	pub fn cx(&self) -> &'cx Context {
		self.cx
	}

	/// Returns the value of the function being called.
	pub fn callee(&self) -> &Object<'cx> {
		&self.callee
	}

	/// Returns a mutable reference to the function being called.
	pub fn callee_mut(&mut self) -> &mut Object<'cx> {
		&mut self.callee
	}

	/// Returns the `this` value of the function.
	/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/this) for more details.
	pub fn this(&self) -> &Value<'cx> {
		&self.this
	}

	/// Returns a mutable reference to the `this` value of the function.
	/// See [Arguments::this] for more details.
	pub fn this_mut(&mut self) -> &mut Value<'cx> {
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

pub struct Accessor<'a, 'cx> {
	args: &'a mut Arguments<'cx>,
	index: usize,
}

impl<'a, 'cx> Accessor<'a, 'cx> {
	/// Returns the number of remaining arguments.
	pub fn len(&self) -> usize {
		self.args.len() - self.index
	}

	pub fn is_empty(&self) -> bool {
		self.args.len() == self.index
	}

	pub fn index(&self) -> usize {
		self.index
	}

	pub fn arg<T: FromValue<'cx>>(&mut self, strict: bool, config: T::Config) -> Option<Result<T>> {
		self.args.values.get(self.index).map(|value| {
			self.index += 1;
			T::from_value(self.args.cx, value, strict, config)
		})
	}

	pub fn args<T: FromValue<'cx>>(&mut self, strict: bool, config: T::Config) -> Result<Vec<T>>
	where
		T::Config: Clone,
	{
		self.args.values[self.index..]
			.iter()
			.map(|value| T::from_value(self.args.cx, value, strict, config.clone()))
			.collect()
	}
}

impl<'cx> Deref for Accessor<'_, 'cx> {
	type Target = Arguments<'cx>;

	fn deref(&self) -> &Self::Target {
		self.args
	}
}

impl<'cx> DerefMut for Accessor<'_, 'cx> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.args
	}
}
