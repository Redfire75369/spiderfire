/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use mozjs::jsapi::{JSFunctionSpec, JSNativeWrapper, JSPropertySpec_Name};

/// Creates a function spec with the given native function, number of arguments and flags.
pub const fn create_function_spec(name: &'static str, call: JSNativeWrapper, nargs: u16, flags: u16) -> JSFunctionSpec {
	JSFunctionSpec {
		name: JSPropertySpec_Name {
			string_: name.as_ptr() as *const i8,
		},
		call,
		nargs,
		flags,
		selfHostedName: ptr::null_mut(),
	}
}

#[macro_export]
macro_rules! function_spec {
	($function:expr, $nargs:expr) => {
		function_spec!($function, stringify!($function), $nargs)
	};
	($function:expr, $name:expr, $nargs:expr) => {
		function_spec!($function, $name, $nargs, $crate::objects::object::JSPROP_CONSTANT)
	};
	($function:expr, $name:expr, $nargs:expr, $flags:expr) => {
		$crate::functions::specs::create_function_spec(
			concat!($name, "\0"),
			::mozjs::jsapi::JSNativeWrapper {
				op: Some($function),
				info: std::ptr::null_mut(),
			},
			$nargs,
			$flags,
		)
	};
}
