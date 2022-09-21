/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::marker::PhantomData;
use std::ptr;
use std::rc::Rc;

use futures::task::AtomicWaker;
use mozjs::jsapi::{ContextOptionsRef, JS_NewGlobalObject, JSAutoRealm, OnNewGlobalHookOption};
use mozjs::rust::{JSEngineHandle, RealmOptions, Runtime as RustRuntime, SIMPLE_GLOBAL_CLASS};

use ion::{Context, ErrorReport, Object};

use crate::event_loop::{EVENT_LOOP, EventLoop};
use crate::event_loop::future::FutureQueue;
use crate::event_loop::macrotasks::MacrotaskQueue;
use crate::event_loop::microtasks::init_microtask_queue;
use crate::globals::{init_globals, init_microtasks, init_timers};
use crate::modules::{init_module_loaders, StandardModules};

pub struct Runtime {
	cx: Context,
	global: Object,
	event_loop: EventLoop,
	#[allow(dead_code)]
	realm: JSAutoRealm,
	#[allow(dead_code)]
	rt: RustRuntime,
}

impl Runtime {
	pub fn cx(&self) -> Context {
		self.cx
	}

	pub fn global(&self) -> Object {
		self.global
	}

	pub async fn run_event_loop(&self) -> Result<(), Option<ErrorReport>> {
		self.event_loop.run_event_loop(self.cx).await
	}
}

#[derive(Copy, Clone, Debug, Default)]
pub struct RuntimeBuilder<T: Default> {
	microtask_queue: bool,
	macrotask_queue: bool,
	modules: bool,
	standard_modules: bool,
	_modules: PhantomData<T>,
}

impl<Std: StandardModules + Default> RuntimeBuilder<Std> {
	pub fn new() -> RuntimeBuilder<Std> {
		RuntimeBuilder::default()
	}

	pub fn macrotask_queue(mut self) -> RuntimeBuilder<Std> {
		self.macrotask_queue = true;
		self
	}

	pub fn microtask_queue(mut self) -> RuntimeBuilder<Std> {
		self.microtask_queue = true;
		self
	}

	pub fn modules(mut self) -> RuntimeBuilder<Std> {
		self.modules = true;
		self
	}

	pub fn standard_modules(mut self) -> RuntimeBuilder<Std> {
		self.standard_modules = true;
		self
	}

	pub fn build(self, engine: JSEngineHandle) -> Runtime {
		let runtime = RustRuntime::new(engine);
		let cx = runtime.cx();
		let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
		let c_options = RealmOptions::default();

		let global = unsafe { JS_NewGlobalObject(cx, &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
		let realm = JSAutoRealm::new(cx, global);
		let mut global = Object::from(global);

		global.set_as(cx, "global", global);
		init_globals(cx, global);

		let mut event_loop = EventLoop {
			futures: None,
			microtasks: None,
			macrotasks: None,
			waker: AtomicWaker::new(),
		};

		if self.microtask_queue {
			event_loop.microtasks = Some(init_microtask_queue(cx));
			init_microtasks(cx, global);
			event_loop.futures = Some(Rc::new(FutureQueue::default()));
		}
		if self.macrotask_queue {
			event_loop.macrotasks = Some(Rc::new(MacrotaskQueue::default()));
			init_timers(cx, global);
		}

		let options = unsafe { &mut *ContextOptionsRef(cx) };
		options.set_topLevelAwait_(true);

		EVENT_LOOP.with(|eloop| {
			let mut eloop = eloop.borrow_mut();
			(*eloop).microtasks = event_loop.microtasks.clone();
			(*eloop).futures = event_loop.futures.clone();
			(*eloop).macrotasks = event_loop.macrotasks.clone();
		});

		if self.modules {
			init_module_loaders(cx);
		}

		if self.standard_modules {
			if self.modules {
				Std::init(cx, &mut global);
			} else {
				Std::init_globals(cx, &mut global);
			}
		}

		Runtime {
			cx,
			rt: runtime,
			event_loop,
			global,
			realm,
		}
	}
}
