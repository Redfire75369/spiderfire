/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use chrono::Duration;
use mozjs::conversions::ConversionBehavior::{Clamp, EnforceRange};
use mozjs::jsapi::{JS_DefineFunctions, JSFunctionSpec, Value};

use ion::{IonContext, IonResult};
use ion::functions::function::IonFunction;
use ion::objects::object::IonObject;

use crate::event_loop::macrotasks::{Macrotask, MACROTASK_QUEUE, TimerMacrotask};

const MINIMUM_DELAY: i64 = 1;
const MINIMUM_DELAY_NESTED: i64 = 4;

fn set_timer(callback: IonFunction, duration: Option<i64>, arguments: Vec<Value>, repeat: bool) -> IonResult<u32> {
	MACROTASK_QUEUE.with(|queue| {
		let queue = queue.borrow();

		let nesting = (*queue).as_ref().unwrap().nesting();
		let minimum = if nesting > 5 { MINIMUM_DELAY_NESTED } else { MINIMUM_DELAY };

		let duration = duration.map(|t| t.max(minimum)).unwrap_or(minimum);
		let timer = TimerMacrotask::new(callback, arguments, repeat, Duration::milliseconds(duration));
		Ok((*queue).as_ref().unwrap().enqueue(Macrotask::Timer(timer), None))
	})
}

fn clear_timer(id: Option<u32>) {
	if let Some(id) = id {
		MACROTASK_QUEUE.with(|queue| {
			(*queue.borrow()).as_ref().unwrap().remove(id);
		});
	}
}

#[js_fn]
fn setTimeout(callback: IonFunction, #[convert(Clamp)] duration: Option<i64>, #[varargs] arguments: Vec<Value>) -> IonResult<u32> {
	set_timer(callback, duration, arguments, false)
}

#[js_fn]
fn setInterval(callback: IonFunction, #[convert(Clamp)] duration: Option<i64>, #[varargs] arguments: Vec<Value>) -> IonResult<u32> {
	set_timer(callback, duration, arguments, true)
}

#[js_fn]
fn clearTimeout(#[convert(EnforceRange)] id: Option<u32>) -> IonResult<()> {
	clear_timer(id);
	Ok(())
}

#[js_fn]
fn clearInterval(#[convert(EnforceRange)] id: Option<u32>) -> IonResult<()> {
	clear_timer(id);
	Ok(())
}

const FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(setTimeout, 1),
	function_spec!(setInterval, 1),
	function_spec!(clearTimeout, 0),
	function_spec!(clearInterval, 0),
	JSFunctionSpec::ZERO,
];

pub unsafe fn define(cx: IonContext, global: IonObject) -> bool {
	rooted!(in(cx) let global = global.raw());
	JS_DefineFunctions(cx, global.handle().into(), FUNCTIONS.as_ptr())
}
