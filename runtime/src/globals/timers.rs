/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use chrono::Duration;
use mozjs::conversions::ConversionBehavior::{Clamp, EnforceRange};
use mozjs::jsapi::{JS_DefineFunctions, JSFunctionSpec, Value};

use ion::{IonContext, IonResult};
use ion::error::IonError;
use ion::functions::function::IonFunction;
use ion::objects::object::IonObject;

use crate::event_loop::EVENT_LOOP;
use crate::event_loop::macrotasks::{Macrotask, TimerMacrotask, UserMacrotask};

const MINIMUM_DELAY: i64 = 1;
const MINIMUM_DELAY_NESTED: i64 = 4;

fn set_timer(callback: IonFunction, duration: Option<i64>, arguments: Vec<Value>, repeat: bool) -> IonResult<u32> {
	EVENT_LOOP.with(|event_loop| {
		if let Some(queue) = (*event_loop.borrow()).macrotasks.clone() {
			let nesting = queue.nesting();
			let minimum = if nesting > 5 { MINIMUM_DELAY_NESTED } else { MINIMUM_DELAY };

			let duration = duration.map(|t| t.max(minimum)).unwrap_or(minimum);
			let timer = TimerMacrotask::new(callback, arguments, repeat, Duration::milliseconds(duration));
			Ok((*queue).enqueue(Macrotask::Timer(timer), None))
		} else {
			Err(IonError::Error(String::from("Macrotask Queue has not been initialised.")))
		}
	})
}

fn clear_timer(id: Option<u32>) -> IonResult<()> {
	if let Some(id) = id {
		EVENT_LOOP.with(|event_loop| {
			if let Some(queue) = (*event_loop.borrow()).macrotasks.clone() {
				queue.remove(id);
				Ok(())
			} else {
				Err(IonError::Error(String::from("Macrotask Queue has not been initialised.")))
			}
		})
	} else {
		Ok(())
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
	clear_timer(id)
}

#[js_fn]
fn clearInterval(#[convert(EnforceRange)] id: Option<u32>) -> IonResult<()> {
	clear_timer(id)
}

#[js_fn]
fn queueMacrotask(callback: IonFunction) -> IonResult<()> {
	EVENT_LOOP.with(|event_loop| {
		if let Some(queue) = (*event_loop.borrow()).macrotasks.clone() {
			queue.enqueue(Macrotask::User(UserMacrotask::new(callback)), None);
			Ok(())
		} else {
			Err(IonError::Error(String::from("Macrotask Queue has not been initialised.")))
		}
	})
}

const FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(setTimeout, 1),
	function_spec!(setInterval, 1),
	function_spec!(clearTimeout, 0),
	function_spec!(clearInterval, 0),
	function_spec!(queueMacrotask, 1),
	JSFunctionSpec::ZERO,
];

pub unsafe fn define(cx: IonContext, global: IonObject) -> bool {
	rooted!(in(cx) let global = global.raw());
	JS_DefineFunctions(cx, global.handle().into(), FUNCTIONS.as_ptr())
}
