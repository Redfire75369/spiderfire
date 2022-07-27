/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::rc::Rc;

use mozjs::glue::{CreateJobQueue, JobQueueTraps};
use mozjs::jsapi::{
	Call, CurrentGlobalOrNull, Handle, HandleValueArray, JobQueueIsEmpty, JobQueueMayNotBeEmpty, JSObject, SetJobQueue, UndefinedHandleValue,
};
use mozjs::jsval::UndefinedValue;

use ion::{Context, ErrorReport, Exception, Function, Object};

use crate::event_loop::EVENT_LOOP;

#[derive(Copy, Clone, Debug)]
pub enum Microtask {
	Promise(Object),
	User(Function),
	None,
}

#[derive(Clone, Debug, Default)]
pub struct MicrotaskQueue {
	queue: RefCell<VecDeque<Microtask>>,
	draining: Cell<bool>,
}

impl Microtask {
	pub fn run(&self, cx: Context) -> bool {
		match self {
			Microtask::Promise(promise) => unsafe {
				rooted!(in(cx) let promise = promise.to_value());
				rooted!(in(cx) let mut rval = UndefinedValue());
				let args = HandleValueArray::new();

				if !Call(cx, UndefinedHandleValue, promise.handle().into(), &args, rval.handle_mut().into()) {
					match Exception::new(cx) {
						Some(e) => ErrorReport::new(e).print(),
						None => return false,
					}
				}
			},
			Microtask::User(callback) => {
				if let Err(report) = callback.call(cx, Object::global(cx), Vec::new()) {
					match report {
						Some(report) => report.print(),
						None => return false,
					}
				}
			}
			_ => (),
		}
		true
	}
}

impl MicrotaskQueue {
	pub fn enqueue(&self, cx: Context, microtask: Microtask) {
		{
			self.queue.borrow_mut().push_back(microtask);
		}
		unsafe { JobQueueMayNotBeEmpty(cx) }
	}

	pub fn run_jobs(&self, cx: Context) -> bool {
		if self.draining.get() {
			return true;
		}

		self.draining.set(true);
		let mut result = true;

		while let Some(microtask) = self.front() {
			let run = microtask.run(cx);
			if !run {
				result = false;
			}
		}

		self.draining.set(false);
		unsafe { JobQueueIsEmpty(cx) };
		result
	}

	fn front(&self) -> Option<Microtask> {
		self.queue.borrow_mut().pop_front()
	}
}

unsafe extern "C" fn get_incumbent_global(_extra: *const c_void, cx: Context) -> *mut JSObject {
	CurrentGlobalOrNull(cx)
}

unsafe extern "C" fn enqueue_promise_job(
	extra: *const c_void, cx: Context, _promise: Handle<*mut JSObject>, job: Handle<*mut JSObject>, _: Handle<*mut JSObject>,
	_: Handle<*mut JSObject>,
) -> bool {
	let queue = &*(extra as *const MicrotaskQueue);
	if !job.is_null() {
		queue.enqueue(cx, Microtask::Promise(Object::from(job.get())))
	} else {
		queue.enqueue(cx, Microtask::None)
	};
	true
}

unsafe extern "C" fn empty(extra: *const c_void) -> bool {
	let queue = &*(extra as *const MicrotaskQueue);
	queue.queue.borrow().is_empty()
}

static JOB_QUEUE_TRAPS: JobQueueTraps = JobQueueTraps {
	getIncumbentGlobal: Some(get_incumbent_global),
	enqueuePromiseJob: Some(enqueue_promise_job),
	empty: Some(empty),
};

pub(crate) fn init_microtask_queue(cx: Context) -> Rc<MicrotaskQueue> {
	let microtask_queue = Rc::new(MicrotaskQueue::default());
	unsafe {
		let queue = CreateJobQueue(&JOB_QUEUE_TRAPS, &*microtask_queue as *const _ as *const c_void);
		SetJobQueue(cx, queue);
	}
	EVENT_LOOP.with(closure!(clone microtask_queue, |event_loop| (*event_loop.borrow_mut()).microtasks = Some(microtask_queue)));
	microtask_queue
}
