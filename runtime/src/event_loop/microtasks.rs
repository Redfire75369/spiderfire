/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::vec_deque::VecDeque;
use std::ffi::c_void;

use mozjs::glue::JobQueueTraps;
use mozjs::jsapi::{CurrentGlobalOrNull, Handle, JobQueueIsEmpty, JobQueueMayNotBeEmpty, JSContext, JSFunction, JSObject};

use ion::{Context, ErrorReport, Function, Object};

use crate::ContextExt;

#[derive(Clone, Debug)]
pub enum Microtask {
	Promise(*mut JSObject),
	User(*mut JSFunction),
	None,
}

#[derive(Clone, Debug, Default)]
pub struct MicrotaskQueue {
	queue: VecDeque<Microtask>,
	draining: bool,
}

impl Microtask {
	pub fn run(&self, cx: &Context) -> Result<(), Option<ErrorReport>> {
		match self {
			Microtask::Promise(job) => {
				let object = cx.root_object(*job);
				let function = Function::from_object(cx, &object).unwrap();

				function.call(cx, &Object::null(cx), &[]).map(|_| ())
			}
			Microtask::User(callback) => {
				let callback = Function::from(cx.root_function(*callback));
				callback.call(cx, &Object::global(cx), &[]).map(|_| ())
			}
			Microtask::None => Ok(()),
		}
	}
}

impl MicrotaskQueue {
	pub fn enqueue(&mut self, cx: &Context, microtask: Microtask) {
		self.queue.push_back(microtask);
		unsafe { JobQueueMayNotBeEmpty(cx.as_ptr()) }
	}

	pub fn run_jobs(&mut self, cx: &Context) -> Result<(), Option<ErrorReport>> {
		if self.draining {
			return Ok(());
		}

		self.draining = true;

		while let Some(microtask) = self.queue.pop_front() {
			microtask.run(cx)?;
		}

		self.draining = false;
		unsafe { JobQueueIsEmpty(cx.as_ptr()) };

		Ok(())
	}

	pub fn is_empty(&self) -> bool {
		self.queue.is_empty()
	}
}

unsafe extern "C" fn get_incumbent_global(_: *const c_void, cx: *mut JSContext) -> *mut JSObject {
	unsafe { CurrentGlobalOrNull(cx) }
}

unsafe extern "C" fn enqueue_promise_job(
	_: *const c_void, cx: *mut JSContext, _: Handle<*mut JSObject>, job: Handle<*mut JSObject>,
	_: Handle<*mut JSObject>, _: Handle<*mut JSObject>,
) -> bool {
	let cx = unsafe { &Context::new_unchecked(cx) };
	let event_loop = unsafe { &mut cx.get_private().event_loop };
	let microtasks = event_loop.microtasks.as_mut().unwrap();
	if !job.is_null() {
		microtasks.enqueue(cx, Microtask::Promise(job.get()))
	} else {
		microtasks.enqueue(cx, Microtask::None)
	};
	true
}

unsafe extern "C" fn empty(extra: *const c_void) -> bool {
	let queue: &MicrotaskQueue = unsafe { &*extra.cast() };
	queue.queue.is_empty()
}

pub(crate) static JOB_QUEUE_TRAPS: JobQueueTraps = JobQueueTraps {
	getIncumbentGlobal: Some(get_incumbent_global),
	enqueuePromiseJob: Some(enqueue_promise_job),
	empty: Some(empty),
};
