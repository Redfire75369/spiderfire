/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Duration, Utc};
use mozjs::jsapi::JSFunction;
use mozjs::jsval::JSVal;

use ion::{Context, ErrorReport, Function, Object, Value};

pub struct SignalMacrotask {
	callback: Box<dyn FnOnce()>,
	terminate: Arc<AtomicBool>,
	scheduled: DateTime<Utc>,
}

impl SignalMacrotask {
	pub fn new(callback: Box<dyn FnOnce()>, terminate: Arc<AtomicBool>, duration: Duration) -> SignalMacrotask {
		SignalMacrotask {
			callback,
			terminate,
			scheduled: Utc::now() + duration,
		}
	}
}

impl Debug for SignalMacrotask {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("SignalMacrotask")
			.field("terminate", &self.terminate.as_ref())
			.field("scheduled", &self.scheduled)
			.finish()
	}
}

#[derive(Debug)]
pub struct TimerMacrotask {
	callback: *mut JSFunction,
	arguments: Vec<JSVal>,
	repeat: bool,
	scheduled: DateTime<Utc>,
	duration: Duration,
	nesting: u8,
}

impl TimerMacrotask {
	pub fn new(callback: Function, arguments: Vec<JSVal>, repeat: bool, duration: Duration) -> TimerMacrotask {
		TimerMacrotask {
			callback: callback.get(),
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

#[derive(Debug)]
pub struct UserMacrotask {
	callback: *mut JSFunction,
	scheduled: DateTime<Utc>,
}

impl UserMacrotask {
	pub fn new(callback: Function) -> UserMacrotask {
		UserMacrotask {
			callback: callback.get(),
			scheduled: Utc::now(),
		}
	}
}

#[derive(Debug)]
pub enum Macrotask {
	Signal(SignalMacrotask),
	Timer(TimerMacrotask),
	User(UserMacrotask),
}

#[derive(Debug, Default)]
pub struct MacrotaskQueue {
	pub(crate) map: HashMap<u32, Macrotask>,
	pub(crate) nesting: u8,
	next: Option<u32>,
	latest: Option<u32>,
}

impl Macrotask {
	pub fn run(self, cx: &Context) -> Result<Option<Macrotask>, Option<ErrorReport>> {
		if let Macrotask::Signal(signal) = self {
			(signal.callback)();
			return Ok(None);
		}
		let (callback, args) = match &self {
			Macrotask::Timer(timer) => (timer.callback, timer.arguments.clone()),
			Macrotask::User(user) => (user.callback, Vec::new()),
			_ => unreachable!(),
		};

		let callback = Function::from(cx.root_function(callback));
		let args: Vec<_> = args.into_iter().map(|value| Value::from(cx.root_value(value))).collect();

		callback.call(cx, &Object::global(cx), args.as_slice()).map(|_| (Some(self)))
	}

	fn terminate(&self) -> bool {
		match self {
			Macrotask::Signal(signal) => signal.terminate.load(Ordering::SeqCst),
			_ => false,
		}
	}

	fn remaining(&self) -> Duration {
		match self {
			Macrotask::Signal(signal) => signal.scheduled - Utc::now(),
			Macrotask::Timer(timer) => timer.scheduled + timer.duration - Utc::now(),
			Macrotask::User(user) => user.scheduled - Utc::now(),
		}
	}
}

impl MacrotaskQueue {
	pub fn run_jobs(&mut self, cx: &Context) -> Result<(), Option<ErrorReport>> {
		self.find_next();
		while let Some(next) = self.next {
			let macrotask = { self.map.remove_entry(&next) };
			if let Some((id, macrotask)) = macrotask {
				let macrotask = macrotask.run(cx)?;

				if let Some(Macrotask::Timer(mut timer)) = macrotask {
					if timer.reset() {
						self.map.insert(id, Macrotask::Timer(timer));
					}
				}
			}
			self.find_next();
		}

		Ok(())
	}

	pub fn enqueue(&mut self, mut macrotask: Macrotask, id: Option<u32>) -> u32 {
		let index = id.unwrap_or_else(|| self.latest.map(|l| l + 1).unwrap_or(0));

		let next = self.next.and_then(|next| self.map.get(&next));
		if let Some(next) = next {
			if macrotask.remaining() < next.remaining() {
				self.set_next(index, &macrotask);
			}
		} else {
			self.set_next(index, &macrotask);
		}

		if let Macrotask::Timer(timer) = &mut macrotask {
			self.nesting += 1;
			timer.nesting = self.nesting;
		}

		self.latest = Some(index);
		self.map.insert(index, macrotask);

		index
	}

	pub fn remove(&mut self, id: u32) {
		if self.map.remove(&id).is_some() {
			if let Some(next) = self.next {
				if next == id {
					self.next = None;
				}
			}
		}
	}

	pub fn find_next(&mut self) {
		let mut next: Option<(u32, &Macrotask)> = None;
		let mut to_remove = Vec::new();
		for (id, macrotask) in &self.map {
			if macrotask.terminate() {
				to_remove.push(*id);
				continue;
			}
			if let Some((_, next_macrotask)) = next {
				if macrotask.remaining() < next_macrotask.remaining() {
					next = Some((*id, macrotask));
				}
			} else if macrotask.remaining() <= Duration::zero() {
				next = Some((*id, macrotask));
			}
		}
		let next = next.map(|(id, _)| id);
		for id in to_remove.iter_mut() {
			self.map.remove(id);
		}
		self.next = next;
	}

	pub fn set_next(&mut self, index: u32, macrotask: &Macrotask) {
		if macrotask.remaining() < Duration::zero() {
			self.next = Some(index);
		}
	}

	pub fn is_empty(&self) -> bool {
		self.map.is_empty()
	}
}
