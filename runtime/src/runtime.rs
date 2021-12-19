/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::marker::PhantomData;
use std::ptr;

use mozjs::jsapi::{JS_NewGlobalObject, JSAutoRealm, OnNewGlobalHookOption};
use mozjs::rust::{JSEngineHandle, RealmOptions, Runtime as RustRuntime, SIMPLE_GLOBAL_CLASS};

use ion::IonContext;
use ion::objects::object::IonObject;

use crate::event_loop::EVENT_LOOP;
use crate::event_loop::macrotasks::init_macrotask_queue;
use crate::event_loop::microtasks::init_microtask_queue;
use crate::globals::{init_globals, init_microtasks, init_timers};
use crate::modules::init_module_loaders;

pub trait StandardModules {
	fn init(cx: IonContext, global: IonObject) -> bool;
}

pub struct Runtime {
	cx: IonContext,
	global: IonObject,
	#[allow(dead_code)]
	realm: JSAutoRealm,
	#[allow(dead_code)]
	rt: RustRuntime,
}

impl Runtime {
	pub fn cx(&self) -> IonContext {
		self.cx
	}

	pub fn global(&self) -> IonObject {
		self.global
	}

	pub fn run_event_loop(&self) -> Result<(), ()> {
		EVENT_LOOP.with(|event_loop| (*event_loop.borrow()).run(self.cx))
	}
}

#[derive(Copy, Clone, Debug, Default)]
pub struct RuntimeBuilder<T: Default> {
	macrotask_queue: bool,
	microtask_queue: bool,
	modules: bool,
	standard_modules: bool,
	_modules: PhantomData<T>,
}

impl<T: Default> RuntimeBuilder<T> {
	pub fn new() -> RuntimeBuilder<T> {
		RuntimeBuilder::default()
	}

	pub fn macrotask_queue(&mut self) -> &mut RuntimeBuilder<T> {
		self.macrotask_queue = true;
		self
	}

	pub fn microtask_queue(&mut self) -> &mut RuntimeBuilder<T> {
		self.microtask_queue = true;
		self
	}

	pub fn modules(&mut self) -> &mut RuntimeBuilder<T> {
		self.modules = true;
		self
	}

	fn build_internal(&mut self, engine: JSEngineHandle) -> Runtime {
		let runtime = RustRuntime::new(engine);
		let cx = runtime.cx();
		let h_options = OnNewGlobalHookOption::FireOnNewGlobalHook;
		let c_options = RealmOptions::default();

		let global = unsafe { JS_NewGlobalObject(cx, &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_options, &*c_options) };
		let realm = JSAutoRealm::new(cx, global);
		let global = IonObject::from(global);

		init_globals(cx, global);

		if self.macrotask_queue {
			init_macrotask_queue();
			init_timers(cx, global);
		}
		if self.microtask_queue {
			init_microtask_queue(cx);
			init_microtasks(cx, global);
		}

		if self.modules {
			init_module_loaders(cx);
		}

		Runtime { cx, rt: runtime, global, realm }
	}
}

impl RuntimeBuilder<()> {
	pub fn build(&mut self, engine: JSEngineHandle) -> Runtime {
		self.build_internal(engine)
	}
}

impl<Std: StandardModules + Default> RuntimeBuilder<Std> {
	pub fn standard_modules(&mut self) -> &mut RuntimeBuilder<Std> {
		self.standard_modules = self.modules;
		self
	}

	pub fn build(&mut self, engine: JSEngineHandle) -> Runtime {
		let rt = self.build_internal(engine);
		if self.standard_modules {
			Std::init(rt.cx(), rt.global());
		}
		rt
	}
}
