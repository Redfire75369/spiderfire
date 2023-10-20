/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::{ClassDefinition, Context, Iterator, Object};

pub mod abort;
pub mod console;
pub mod encoding;
#[cfg(feature = "fetch")]
pub mod fetch;
pub mod microtasks;
pub mod timers;
pub mod url;

pub fn init_globals(cx: &Context, global: &mut Object) -> bool {
	let result = console::define(cx, global) && encoding::define(cx, global) && url::define(cx, global) && Iterator::init_class(cx, global).0;
	#[cfg(feature = "fetch")]
	{
		result && fetch::define(cx, global)
	}
	#[cfg(not(feature = "fetch"))]
	{
		result
	}
}

pub fn init_timers(cx: &Context, global: &mut Object) -> bool {
	timers::define(cx, global) && abort::define(cx, global)
}

pub fn init_microtasks(cx: &Context, global: &mut Object) -> bool {
	microtasks::define(cx, global)
}
