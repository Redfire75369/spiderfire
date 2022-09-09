/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use mozjs::jsval::JSVal;

use ion::{Context, ErrorReport, Function, Object};

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
	pub(crate) nesting: Cell<u8>,
	next: Cell<Option<u32>>,
	latest: Cell<Option<u32>>,
}

impl Macrotask {
	pub fn run(&self, cx: Context) -> Result<(), Option<ErrorReport>> {
		let (callback, args) = match self {
			Macrotask::Timer(timer) => (timer.callback, timer.arguments.clone()),
			Macrotask::User(user) => (user.callback, Vec::new()),
		};

		return callback.call(cx, Object::global(cx), args).map(|_| ());
	}

	fn remaining(&self) -> Duration {
		match *self {
			Macrotask::Timer(ref timer) => timer.scheduled + timer.duration - Utc::now(),
			Macrotask::User(ref user) => user.scheduled - Utc::now(),
		}
	}
}

impl MacrotaskQueue {
	pub fn run_all(&self, cx: Context) -> Result<(), Option<ErrorReport>> {
		let mut is_empty = { self.map.borrow().is_empty() };

		while !is_empty {
			if let Some(next) = self.next.get() {
				let macrotask = { self.map.borrow().get(&next).cloned() };
				if let Some(macrotask) = macrotask {
					macrotask.run(cx)?;

					let mut queue = self.map.borrow_mut();
					if let Entry::Occupied(mut entry) = queue.entry(next) {
						let mut to_remove = true;
						if let Macrotask::Timer(ref mut timer) = entry.get_mut() {
							if timer.reset() {
								to_remove = false;
							}
						}

						if to_remove {
							entry.remove();
						}
					}
				}
			}
			self.find_next();
			is_empty = self.map.borrow().is_empty();
		}

		Ok(())
	}

	pub fn enqueue(&self, mut macrotask: Macrotask, id: Option<u32>) -> u32 {
		let index = id.unwrap_or_else(|| self.latest.get().map(|l| l + 1).unwrap_or(0));

		let mut queue = self.map.borrow_mut();
		let next = self.next.get().and_then(|next| (*queue).get(&next));
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
		queue.insert(index, macrotask);

		index
	}

	pub fn remove(&self, id: u32) {
		if self.map.borrow_mut().remove(&id).is_some() {
			if let Some(next) = self.next.get() {
				if next == id {
					self.next.set(None);
				}
			}
		}
	}

	pub fn find_next(&self) {
		let mut next: Option<(u32, &Macrotask)> = None;
		let queue = self.map.borrow();
		for (id, macrotask) in &*queue {
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
			self.next.set(Some(index));
		}
	}

	pub fn is_empty(&self) -> bool {
		self.map.borrow().is_empty()
	}
}
