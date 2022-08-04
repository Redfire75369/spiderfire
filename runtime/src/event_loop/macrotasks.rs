/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use chrono::{DateTime, Duration, Utc};
use mozjs::jsval::JSVal;

use ion::{Context, Function, Object};

use crate::event_loop::EVENT_LOOP;

#[derive(Clone, Debug)]
pub struct TimerMacrotask {
	callback: Function,
	arguments: Vec<JSVal>,
	repeat: bool,
	scheduled: DateTime<Utc>,
	duration: Duration,
	nesting: u8,
}

impl TimerMacrotask {
	pub fn new(callback: Function, arguments: Vec<JSVal>, repeat: bool, duration: Duration) -> TimerMacrotask {
		TimerMacrotask {
			callback,
			arguments,
			repeat,
			duration,
			scheduled: Utc::now(),
			nesting: 0,
		}
	}

	pub fn reset(&mut self) -> bool {
		if self.repeat {
			self.scheduled = Utc::now();
		}
		self.repeat
	}
}

#[derive(Clone, Debug)]
pub struct UserMacrotask {
	callback: Function,
	scheduled: DateTime<Utc>,
}

impl UserMacrotask {
	pub fn new(callback: Function) -> UserMacrotask {
		UserMacrotask { callback, scheduled: Utc::now() }
	}
}

#[derive(Clone, Debug)]
pub enum Macrotask {
	Timer(TimerMacrotask),
	User(UserMacrotask),
}

#[derive(Clone, Debug, Default)]
pub struct MacrotaskQueue {
	pub(crate) map: RefCell<HashMap<u32, Macrotask>>,
	next: Cell<Option<u32>>,
	nesting: Cell<u8>,
	latest: Cell<Option<u32>>,
}

impl Macrotask {
	pub fn run(&self, cx: Context) -> bool {
		let (callback, args) = match self {
			Macrotask::Timer(timer) => (timer.callback, timer.arguments.clone()),
			Macrotask::User(user) => (user.callback, Vec::new()),
		};

		if let Err(report) = callback.call(cx, Object::global(cx), args) {
			match report {
				Some(report) => println!("{}", report),
				None => return false,
			}
		}
		true
	}

	fn remaining(&self) -> Duration {
		match *self {
			Macrotask::Timer(ref timer) => timer.scheduled + timer.duration - Utc::now(),
			Macrotask::User(ref user) => user.scheduled - Utc::now(),
		}
	}
}

impl MacrotaskQueue {
	pub fn enqueue(&self, mut macrotask: Macrotask, id: Option<u32>) -> u32 {
		let index = id.unwrap_or_else(|| self.latest.get().map(|l| l + 1).unwrap_or(0));
		let mut macrotasks = self.map.borrow_mut();

		let next = self.next.get().and_then(|next| (*macrotasks).get(&next));
		if let Some(next) = next {
			if macrotask.remaining() < next.remaining() {
				self.set_next(index, &macrotask);
			}
		} else {
			self.set_next(index, &macrotask);
		}

		if let Macrotask::Timer(ref mut timer) = macrotask {
			self.nesting.set(self.nesting.get() + 1);
			timer.nesting = self.nesting.get();
		}
		self.latest.set(Some(index));

		macrotasks.insert(index, macrotask);

		index
	}

	pub fn remove(&self, id: u32) {
		if self.map.borrow_mut().remove(&id).is_some() {
			if let Some(next) = self.next.get() {
				if next == id {
					self.next.set(None)
				}
			}
		}
	}

	pub fn find_next(&self) {
		let macrotasks = self.map.borrow_mut();
		let mut next: Option<(u32, &Macrotask)> = None;
		for (id, macrotask) in &*macrotasks {
			if let Some((_, next_macrotask)) = next {
				if macrotask.remaining() < next_macrotask.remaining() {
					next = Some((*id, macrotask));
				}
			} else if macrotask.remaining() <= Duration::zero() {
				next = Some((*id, macrotask));
			}
		}
		self.next.set(next.map(|(id, _)| id));
	}

	pub fn set_next(&self, index: u32, macrotask: &Macrotask) {
		if macrotask.remaining() < Duration::zero() {
			self.next.set(Some(index))
		}
	}

	pub fn next(&self) -> Option<u32> {
		self.next.get()
	}

	pub fn nesting(&self) -> u8 {
		self.nesting.get()
	}
}

pub(crate) fn init_macrotask_queue() -> Rc<MacrotaskQueue> {
	let macrotask_queue = Rc::new(MacrotaskQueue::default());
	EVENT_LOOP.with(closure!(clone macrotask_queue, |event_loop| (*event_loop.borrow_mut()).macrotasks = Some(macrotask_queue)));
	macrotask_queue
}
