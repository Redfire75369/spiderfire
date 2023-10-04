/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use chrono::Duration;
use mozjs::conversions::ConversionBehavior::{Clamp, EnforceRange};
use mozjs::jsapi::JSFunctionSpec;
use mozjs::jsval::JSVal;

use ion::{Context, Error, Function, Object, Result};

use crate::event_loop::EVENT_LOOP;
use crate::event_loop::macrotasks::{Macrotask, TimerMacrotask, UserMacrotask};

const MINIMUM_DELAY: i32 = 1;
const MINIMUM_DELAY_NESTED: i32 = 4;

fn set_timer(callback: Function, duration: Option<i32>, arguments: Vec<JSVal>, repeat: bool) -> Result<u32> {
	EVENT_LOOP.with(|event_loop| {
		if let Some(queue) = event_loop.borrow_mut().macrotasks.as_mut() {
			let minimum = if queue.nesting.get() > 5 { MINIMUM_DELAY_NESTED } else { MINIMUM_DELAY };

			let duration = duration.map(|t| t.max(minimum)).unwrap_or(minimum);
			let timer = TimerMacrotask::new(callback, arguments, repeat, Duration::milliseconds(duration as i64));
			Ok(queue.enqueue(Macrotask::Timer(timer), None))
		} else {
			Err(Error::new("Macrotask Queue has not been initialised.", None))
		}
	})
}

fn clear_timer(id: Option<u32>) -> Result<()> {
	if let Some(id) = id {
		EVENT_LOOP.with(|event_loop| {
			if let Some(queue) = event_loop.borrow_mut().macrotasks.as_mut() {
				queue.remove(id);
				Ok(())
			} else {
				Err(Error::new("Macrotask Queue has not been initialised.", None))
			}
		})
	} else {
		Ok(())
	}
}

#[js_fn]
fn setTimeout(callback: Function, #[ion(convert = Clamp)] duration: Option<i32>, #[ion(varargs)] arguments: Vec<JSVal>) -> Result<u32> {
	set_timer(callback, duration, arguments, false)
}

#[js_fn]
fn setInterval(callback: Function, #[ion(convert = Clamp)] duration: Option<i32>, #[ion(varargs)] arguments: Vec<JSVal>) -> Result<u32> {
	set_timer(callback, duration, arguments, true)
}

#[js_fn]
fn clearTimeout(#[ion(convert = EnforceRange)] id: Option<u32>) -> Result<()> {
	clear_timer(id)
}

#[js_fn]
fn clearInterval(#[ion(convert = EnforceRange)] id: Option<u32>) -> Result<()> {
	clear_timer(id)
}

#[js_fn]
fn queueMacrotask(callback: Function) -> Result<()> {
	EVENT_LOOP.with(|event_loop| {
		if let Some(queue) = event_loop.borrow_mut().macrotasks.as_mut() {
			queue.enqueue(Macrotask::User(UserMacrotask::new(callback)), None);
			Ok(())
		} else {
			Err(Error::new("Macrotask Queue has not been initialised.", None))
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

pub fn define<'cx>(cx: &'cx Context, global: &'cx mut Object) -> bool {
	unsafe { global.define_methods(cx, FUNCTIONS) }
}
