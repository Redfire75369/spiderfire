/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::JSFunctionSpec;

use ion::{Context, Error, Function, Object, Result};
use ion::flags::PropertyFlags;

use crate::event_loop::EVENT_LOOP;
use crate::event_loop::microtasks::Microtask;

#[js_fn]
fn queueMicrotask(cx: &Context, callback: Function) -> Result<()> {
	EVENT_LOOP.with(|event_loop| {
		if let Some(queue) = event_loop.borrow().microtasks.clone() {
			queue.enqueue(**cx, Microtask::User(**callback));
			Ok(())
		} else {
			Err(Error::new("Microtask Queue has not been initialised.", None))
		}
	})
}

const FUNCTION: JSFunctionSpec = function_spec!(queueMicrotask, 0);

pub fn define<'cx>(cx: &'cx Context, global: &mut Object<'cx>) -> bool {
	global.define_as(
		cx,
		"queueMicrotask",
		&Function::from_spec(cx, &FUNCTION),
		PropertyFlags::CONSTANT_ENUMERATED,
	)
}
