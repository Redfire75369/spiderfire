/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use mozjs::jsapi::{JSFunctionSpec, JSNativeWrapper, JSPropertySpec_Name};

use crate::flags::PropertyFlags;
use crate::symbol::WellKnownSymbolCode;

/// Creates a [function spec](JSFunctionSpec) with the given name, native function, number of arguments and flags.
pub const fn create_function_spec(
	name: &'static str, func: JSNativeWrapper, nargs: u16, flags: PropertyFlags,
) -> JSFunctionSpec {
	JSFunctionSpec {
		name: JSPropertySpec_Name { string_: name.as_ptr().cast() },
		call: func,
		nargs,
		flags: flags.bits(),
		selfHostedName: ptr::null_mut(),
	}
}

/// Creates a [function spec](JSFunctionSpec) with the given symbol, native function, number of arguments and flags.
pub const fn create_function_spec_symbol(
	symbol: WellKnownSymbolCode, func: JSNativeWrapper, nargs: u16, flags: PropertyFlags,
) -> JSFunctionSpec {
	JSFunctionSpec {
		name: JSPropertySpec_Name { symbol_: symbol as u32 as usize + 1 },
		call: func,
		nargs,
		flags: flags.bits(),
		selfHostedName: ptr::null_mut(),
	}
}

#[cfg(feature = "macros")]
#[macro_export(local_inner_macros)]
macro_rules! function_spec {
	($function:expr, $name:expr, $nargs:expr, $flags:expr) => {
		$crate::spec::create_function_spec(
			::std::concat!($name, "\0"),
			::mozjs::jsapi::JSNativeWrapper {
				op: Some($function),
				info: ::std::ptr::null_mut(),
			},
			$nargs,
			$flags,
		)
	};
	($function:expr, $name:expr, $nargs:expr) => {
		function_spec!(
			$function,
			$name,
			$nargs,
			$crate::flags::PropertyFlags::CONSTANT_ENUMERATED
		)
	};
	($function:expr, $nargs:expr) => {
		function_spec!($function, ::std::stringify!($function), $nargs)
	};
}

#[cfg(feature = "macros")]
#[macro_export(local_inner_macros)]
macro_rules! function_spec_symbol {
	($function:expr, $symbol:expr, $nargs:expr, $flags:expr) => {
		$crate::spec::create_function_spec_symbol(
			$symbol,
			::mozjs::jsapi::JSNativeWrapper {
				op: Some($function),
				info: ::std::ptr::null_mut(),
			},
			$nargs,
			$flags,
		)
	};
	($function:expr, $symbol:expr, $nargs:expr) => {
		create_function_spec_symbol!(
			$function,
			$symbol,
			$nargs,
			$crate::flags::PropertyFlags::CONSTANT_ENUMERATED
		)
	};
}
