/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::process;
use std::rc::Rc;

use mozjs::glue::{CreateJobQueue, JobQueueTraps};
use mozjs::jsapi::{Call, CurrentGlobalOrNull, Handle, HandleValueArray, JobQueueIsEmpty, JobQueueMayNotBeEmpty, SetJobQueue, UndefinedHandleValue};
use mozjs::jsval::UndefinedValue;

use ion::exception::{ErrorReport, Exception};
use ion::IonContext;
use ion::objects::object::{IonObject, IonRawObject};

#[derive(Copy, Clone, Debug)]
enum Microtask {
	Promise(IonObject),
	None,
}

#[derive(Clone, Debug, Default)]
struct MicrotaskQueue {
	microtasks: RefCell<Vec<Microtask>>,
	draining: Cell<bool>,
}

impl MicrotaskQueue {
	pub fn enqueue_microtask(&self, cx: IonContext, microtask: Microtask, _global: IonObject) {
		{
			self.microtasks.borrow_mut().push(microtask);
		}
		unsafe { JobQueueMayNotBeEmpty(cx) }
		if self.run_jobs(cx).is_err() {
			eprintln!("Unknown Error during Microtask");
			process::exit(1);
		}
	}

	pub fn run_jobs(&self, cx: IonContext) -> Result<(), ()> {
		if self.draining.get() {
			Ok(())
		} else {
			self.draining.set(true);

			let mut ret = Ok(());
			let args = HandleValueArray::new();
			rooted!(in(cx) let mut rval = UndefinedValue());

			let queue = self.microtasks.borrow();

			let mut index = 0;
			while index < queue.len() {
				let microtask = queue[index];
				index += 1;

				unsafe {
					match microtask {
						Microtask::Promise(ref promise) => {
							rooted!(in(cx) let promise = promise.to_value());
							if !Call(cx, UndefinedHandleValue, promise.handle().into(), &args, rval.handle_mut().into()) {
								match Exception::new(cx) {
									Some(e) => ErrorReport::new(e).print(),
									None => ret = Err(()),
								}
							}
						}
						Microtask::None => (),
					}
				}
			}

			self.clear();
			self.draining.set(false);
			unsafe { JobQueueIsEmpty(cx) };
			ret
		}
	}

	pub fn clear(&self) {
		*self.microtasks.borrow_mut() = vec![];
	}
}

unsafe extern "C" fn get_incumbent_global(_extra: *const c_void, cx: IonContext) -> IonRawObject {
	CurrentGlobalOrNull(cx)
}

unsafe extern "C" fn enqueue_promise_job(
	extra: *const c_void, cx: IonContext, _promise: Handle<IonRawObject>, job: Handle<IonRawObject>, _: Handle<IonRawObject>,
	incumbent_global: Handle<IonRawObject>,
) -> bool {
	let queue = &*(extra as *const MicrotaskQueue);
	let global = IonObject::from(incumbent_global.get());
	if !job.is_null() {
		queue.enqueue_microtask(cx, Microtask::Promise(IonObject::from(job.get())), global)
	} else {
		queue.enqueue_microtask(cx, Microtask::None, global)
	};
	true
}

unsafe extern "C" fn empty(extra: *const c_void) -> bool {
	let queue = &*(extra as *const MicrotaskQueue);
	queue.clear();
	true
}

static JOB_QUEUE_TRAPS: JobQueueTraps = JobQueueTraps {
	getIncumbentGlobal: Some(get_incumbent_global),
	enqueuePromiseJob: Some(enqueue_promise_job),
	empty: Some(empty),
};

pub fn init_microtask_queue(cx: IonContext) {
	let microtask_queue = Rc::new(MicrotaskQueue::default());
	unsafe {
		let queue = CreateJobQueue(&JOB_QUEUE_TRAPS, &*microtask_queue as *const _ as *const c_void);
		SetJobQueue(cx, queue);
	}
}
