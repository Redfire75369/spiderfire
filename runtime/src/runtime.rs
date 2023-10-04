/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::c_void;
use std::rc::Rc;

use mozjs::jsapi::{ContextOptionsRef, JSAutoRealm};

use ion::{Context, ErrorReport, Object};
use ion::module::{init_module_loader, ModuleLoader};
use ion::objects::default_new_global;

use crate::event_loop::{EVENT_LOOP, EventLoop};
use crate::event_loop::future::FutureQueue;
use crate::event_loop::macrotasks::MacrotaskQueue;
use crate::event_loop::microtasks::init_microtask_queue;
use crate::globals::{init_globals, init_microtasks, init_timers};
use crate::modules::StandardModules;

#[derive(Default)]
pub struct ContextPrivate {}

pub trait ContextExt {
	#[allow(clippy::mut_from_ref)]
	fn get_private(&self) -> &mut ContextPrivate;
}

impl ContextExt for Context<'_> {
	fn get_private(&self) -> &mut ContextPrivate {
		unsafe {
			let private = self.get_raw_private() as *mut ContextPrivate;
			&mut *private
		}
	}
}

pub struct Runtime<'c, 'cx> {
	global: Object<'cx>,
	cx: &'cx Context<'c>,
	event_loop: EventLoop,
	#[allow(dead_code)]
	realm: JSAutoRealm,
}

impl<'cx> Runtime<'_, 'cx> {
	pub fn cx(&self) -> &Context {
		self.cx
	}

	pub fn global(&self) -> &Object<'cx> {
		&self.global
	}

	pub fn global_mut(&mut self) -> &mut Object<'cx> {
		&mut self.global
	}

	pub async fn run_event_loop(&self) -> Result<(), Option<ErrorReport>> {
		self.event_loop.run_event_loop(self.cx).await
	}
}

impl Drop for Runtime<'_, '_> {
	fn drop(&mut self) {
		let private = self.cx.get_raw_private() as *mut ContextPrivate;
		if !private.is_null() {
			let _ = unsafe { Box::from_raw(private) };
		}
		let inner_private = self.cx.get_inner_data();
		if !inner_private.is_null() {
			let _ = unsafe { Box::from_raw(inner_private) };
		}
	}
}

#[derive(Copy, Clone, Debug)]
pub struct RuntimeBuilder<ML: ModuleLoader + 'static = (), Std: StandardModules = ()> {
	microtask_queue: bool,
	macrotask_queue: bool,
	modules: Option<ML>,
	standard_modules: Option<Std>,
}

impl<ML: ModuleLoader + 'static, Std: StandardModules> RuntimeBuilder<ML, Std> {
	pub fn new() -> RuntimeBuilder<ML, Std> {
		RuntimeBuilder::default()
	}

	pub fn macrotask_queue(mut self) -> RuntimeBuilder<ML, Std> {
		self.macrotask_queue = true;
		self
	}

	pub fn microtask_queue(mut self) -> RuntimeBuilder<ML, Std> {
		self.microtask_queue = true;
		self
	}

	pub fn modules(mut self, loader: ML) -> RuntimeBuilder<ML, Std> {
		self.modules = Some(loader);
		self
	}

	pub fn standard_modules(mut self, standard_modules: Std) -> RuntimeBuilder<ML, Std> {
		self.standard_modules = Some(standard_modules);
		self
	}

	pub fn build<'c, 'cx>(self, cx: &'cx Context<'c>) -> Runtime<'c, 'cx> {
		let mut global = default_new_global(cx);
		let realm = JSAutoRealm::new(cx.as_ptr(), global.handle().get());

		let global_obj = global.handle().get();
		global.set_as(cx, "global", &global_obj);
		init_globals(cx, &mut global);

		let private = Box::<ContextPrivate>::default();
		cx.set_raw_private(Box::into_raw(private) as *mut c_void);

		let mut event_loop = EventLoop {
			futures: None,
			microtasks: None,
			macrotasks: None,
		};

		if self.microtask_queue {
			event_loop.microtasks = Some(init_microtask_queue(cx));
			init_microtasks(cx, &mut global);
			event_loop.futures = Some(Rc::new(FutureQueue::default()));
		}
		if self.macrotask_queue {
			event_loop.macrotasks = Some(Rc::new(MacrotaskQueue::default()));
			init_timers(cx, &mut global);
		}

		let _options = unsafe { &mut *ContextOptionsRef(cx.as_ptr()) };

		EVENT_LOOP.with(|eloop| {
			let mut eloop = eloop.borrow_mut();
			eloop.microtasks = event_loop.microtasks.clone();
			eloop.futures = event_loop.futures.clone();
			eloop.macrotasks = event_loop.macrotasks.clone();
		});

		let has_loader = self.modules.is_some();
		if let Some(loader) = self.modules {
			init_module_loader(cx, loader);
		}

		if let Some(standard_modules) = self.standard_modules {
			if has_loader {
				standard_modules.init(cx, &mut global);
			} else {
				standard_modules.init_globals(cx, &mut global);
			}
		}

		Runtime { global, cx, event_loop, realm }
	}
}

impl<ML: ModuleLoader + 'static, Std: StandardModules> Default for RuntimeBuilder<ML, Std> {
	fn default() -> RuntimeBuilder<ML, Std> {
		RuntimeBuilder {
			microtask_queue: false,
			macrotask_queue: false,
			modules: None,
			standard_modules: None,
		}
	}
}
