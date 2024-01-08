/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::{CallArgs, JS_GetFunctionId, JS_GetObjectFunction};
use mozjs::jsval::JSVal;

use crate::{Context, Error, ErrorKind, Local, Object, Result, Value};
use crate::conversions::FromValue;
use crate::function::{Opt, Rest};

/// Represents Arguments to a [JavaScript Function](crate::Function).
/// Wrapper around [CallArgs] to provide lifetimes and root all arguments.
pub struct Arguments<'cx> {
	cx: &'cx Context,
	args: usize,
	callee: Object<'cx>,
	call_args: CallArgs,
}

impl<'cx> Arguments<'cx> {
	pub unsafe fn new(cx: &'cx Context, argc: u32, vp: *mut JSVal) -> Arguments<'cx> {
		unsafe {
			let call_args = CallArgs::from_vp(vp, argc);
			let callee = cx.root_object(call_args.callee()).into();

			Arguments {
				cx,
				args: argc as usize,
				callee,
				call_args,
			}
		}
	}

	/// Checks if the expected minimum number of arguments were passed.
	pub fn check_args(&self, cx: &Context, min: usize) -> Result<()> {
		if self.args < min {
			let func = crate::String::from(unsafe {
				Local::from_marked(&JS_GetFunctionId(JS_GetObjectFunction(self.callee.handle().get())))
			})
			.to_owned(cx);

			return Err(Error::new(
				&format!(
					"Error while calling `{func}()` with {} argument(s) while {min} were expected.",
					self.args
				),
				ErrorKind::Normal,
			));
		}
		Ok(())
	}

	/// Returns the number of arguments.
	pub fn len(&self) -> usize {
		self.args
	}

	/// Returns `true` if there are no arguments.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Returns the context associated with the arguments.
	pub fn cx(&self) -> &'cx Context {
		self.cx
	}

	/// Returns the value of the function being called.
	pub fn callee(&self) -> Object<'cx> {
		Object::from(Local::from_handle(self.callee.handle()))
	}

	/// Returns the `this` value of the function.
	/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/this) for more details.
	pub fn this(&self) -> Value<'cx> {
		Value::from(unsafe { Local::from_raw_handle(self.call_args.thisv()) })
	}

	/// Returns the return value of the function.
	/// This value can be modified to change the return value.
	pub fn rval(&mut self) -> Value<'cx> {
		Value::from(unsafe { Local::from_raw_handle_mut(self.call_args.rval()) })
	}

	/// Returns the argument at a given index.
	pub fn value(&self, index: usize) -> Option<Value<'cx>> {
		if index < self.len() {
			return Some(Value::from(unsafe {
				Local::from_raw_handle(self.call_args.get(index as u32))
			}));
		}
		None
	}

	/// Returns `true` if the function was called with `new`.
	pub fn is_constructing(&self) -> bool {
		self.call_args.constructing_()
	}

	/// Returns `true` if the function ignores the return value.
	pub fn ignores_return_value(&self) -> bool {
		self.call_args.ignoresReturnValue_()
	}

	pub fn call_args(&self) -> &CallArgs {
		&self.call_args
	}

	pub fn access<'a>(&'a mut self) -> Accessor<'a, 'cx> {
		Accessor { args: self, index: 0 }
	}
}

pub struct Accessor<'a, 'cx> {
	args: &'a mut Arguments<'cx>,
	index: usize,
}

impl<'cx> Accessor<'_, 'cx> {
	/// Returns the number of arguments remaining.
	pub fn len(&self) -> usize {
		self.args.len() - self.index
	}

	/// Returns `true` if there are no arguments remaining.
	pub fn is_empty(&self) -> bool {
		self.index == 0
	}

	/// Returns the context associated with the arguments.
	pub fn cx(&self) -> &'cx Context {
		self.args.cx()
	}

	/// Returns the value of the function being called.
	pub fn callee(&self) -> Object<'cx> {
		self.args.callee()
	}

	/// Returns the `this` value of the function.
	/// Refer to [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/this) for more details.
	pub fn this(&self) -> Value<'cx> {
		self.args.this()
	}

	/// Returns the argument at the current index.
	///
	/// ### Panics
	/// Panics if there are no arguments remaining.
	pub fn value(&mut self) -> Value<'cx> {
		assert!(self.index < self.args.len());
		let arg = self.args.value(self.index).unwrap();
		self.index += 1;
		arg
	}

	/// Returns `true` if the function was called with `new`.
	pub fn is_constructing(&self) -> bool {
		self.args.is_constructing()
	}

	/// Returns `true` if the function ignores the return value.
	pub fn ignores_return_value(&self) -> bool {
		self.args.ignores_return_value()
	}
}

pub trait FromArgument<'a, 'cx>: Sized {
	type Config;

	/// Converts from an argument.
	fn from_argument(accessor: &'a mut Accessor<'_, 'cx>, config: Self::Config) -> Result<Self>;
}

impl<'cx> FromArgument<'_, 'cx> for &'cx Context {
	type Config = ();

	fn from_argument(accessor: &mut Accessor<'_, 'cx>, _: ()) -> Result<&'cx Context> {
		Ok(accessor.cx())
	}
}

impl<'a, 'cx> FromArgument<'a, 'cx> for &'a mut Arguments<'cx> {
	type Config = ();

	fn from_argument(accessor: &'a mut Accessor<'_, 'cx>, _: ()) -> Result<&'a mut Arguments<'cx>> {
		Ok(accessor.args)
	}
}

impl<'cx, T: FromValue<'cx>> FromArgument<'_, 'cx> for T {
	type Config = T::Config;

	fn from_argument(accessor: &mut Accessor<'_, 'cx>, config: T::Config) -> Result<T> {
		T::from_value(accessor.cx(), &accessor.value(), false, config)
	}
}

impl<'cx, T: FromValue<'cx>> FromArgument<'_, 'cx> for Opt<T> {
	type Config = T::Config;

	fn from_argument(accessor: &mut Accessor<'_, 'cx>, config: Self::Config) -> Result<Opt<T>> {
		if accessor.is_empty() {
			Ok(Opt(None))
		} else {
			T::from_value(accessor.cx(), &accessor.value(), false, config).map(Some).map(Opt)
		}
	}
}

impl<'cx, T: FromValue<'cx>> FromArgument<'_, 'cx> for Rest<T>
where
	T::Config: Clone,
{
	type Config = T::Config;

	fn from_argument(accessor: &mut Accessor<'_, 'cx>, config: Self::Config) -> Result<Rest<T>> {
		(0..accessor.len())
			.map(|_| T::from_value(accessor.cx(), &accessor.value(), false, config.clone()))
			.collect::<Result<Box<[_]>>>()
			.map(Rest)
	}
}
