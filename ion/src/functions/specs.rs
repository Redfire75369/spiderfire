/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use mozjs::jsapi::{JSFunctionSpec, JSNativeWrapper, JSPROP_ENUMERATE, JSPROP_PERMANENT, JSPROP_READONLY, JSPropertySpec_Name};

pub const NULL_SPEC: JSFunctionSpec = JSFunctionSpec {
	name: JSPropertySpec_Name { string_: ptr::null_mut() },
	call: JSNativeWrapper {
		op: None,
		info: ptr::null_mut(),
	},
	nargs: 0,
	flags: 0,
	selfHostedName: ptr::null_mut(),
};

pub const fn create_function_spec(name: &str, call: JSNativeWrapper, nargs: u16) -> JSFunctionSpec {
	JSFunctionSpec {
		name: JSPropertySpec_Name {
			string_: name.as_ptr() as *const i8,
		},
		call,
		nargs,
		flags: (JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT) as u16,
		selfHostedName: ptr::null_mut(),
	}
}
