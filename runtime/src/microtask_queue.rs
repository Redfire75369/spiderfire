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
use mozjs::jsapi::{Call, CurrentGlobalOrNull, Handle, HandleValueArray, JobQueueIsEmpty, JobQueueMayNotBeEmpty, SetJobQueue, UndefinedHandleValue};
use mozjs::jsval::UndefinedValue;

use ion::exception::{ErrorReport, Exception};
use ion::IonContext;
use ion::objects::object::{IonObject, IonRawObject};

#[derive(Copy, Clone, Debug)]
pub enum Microtask {
	Promise(IonObject),
	None,
}

#[derive(Clone, Debug, Default)]
pub struct MicrotaskQueue {
	microtasks: RefCell<VecDeque<Microtask>>,
	draining: Cell<bool>,
}

impl MicrotaskQueue {
	pub fn enqueue(&self, cx: IonContext, microtask: Microtask) {
		{
			self.microtasks.borrow_mut().push_back(microtask);
		}
		unsafe { JobQueueMayNotBeEmpty(cx) }
	}

	pub fn run_jobs(&self, cx: IonContext) -> Result<(), ()> {
		if self.draining.get() {
			return Ok(());
		}

		self.draining.set(true);

		let mut ret = Ok(());
		let args = HandleValueArray::new();
		rooted!(in(cx) let mut rval = UndefinedValue());

		while let Some(microtask) = self.microtasks.borrow_mut().pop_front() {
			match microtask {
				Microtask::Promise(promise) => unsafe {
					rooted!(in(cx) let promise = promise.to_value());
					if !Call(cx, UndefinedHandleValue, promise.handle().into(), &args, rval.handle_mut().into()) {
						match Exception::new(cx) {
							Some(e) => ErrorReport::new(e).print(),
							None => ret = Err(()),
						}
					}
				},
				_ => (),
			}
		}

		*self.microtasks.borrow_mut() = VecDeque::new();

		self.draining.set(false);
		unsafe { JobQueueIsEmpty(cx) };
		ret
	}
}

unsafe extern "C" fn get_incumbent_global(_extra: *const c_void, cx: IonContext) -> IonRawObject {
	CurrentGlobalOrNull(cx)
}

unsafe extern "C" fn enqueue_promise_job(
	extra: *const c_void, cx: IonContext, _promise: Handle<IonRawObject>, job: Handle<IonRawObject>, _: Handle<IonRawObject>, _: Handle<IonRawObject>,
) -> bool {
	let queue = &*(extra as *const MicrotaskQueue);
	if !job.is_null() {
		queue.enqueue(cx, Microtask::Promise(IonObject::from(job.get())))
	} else {
		queue.enqueue(cx, Microtask::None)
	};
	true
}

unsafe extern "C" fn empty(extra: *const c_void) -> bool {
	let queue = &*(extra as *const MicrotaskQueue);
	queue.microtasks.borrow().is_empty()
}

static JOB_QUEUE_TRAPS: JobQueueTraps = JobQueueTraps {
	getIncumbentGlobal: Some(get_incumbent_global),
	enqueuePromiseJob: Some(enqueue_promise_job),
	empty: Some(empty),
};

pub fn init_microtask_queue(cx: IonContext) -> Rc<MicrotaskQueue> {
	let microtask_queue = Rc::new(MicrotaskQueue::default());
	unsafe {
		let queue = CreateJobQueue(&JOB_QUEUE_TRAPS, &*microtask_queue as *const _ as *const c_void);
		SetJobQueue(cx, queue);
	}
	microtask_queue.clone()
}
