/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::rc::Rc;
use std::task;
use std::task::Poll;

use futures::future::poll_fn;
use futures::task::AtomicWaker;

use ion::{Context, ErrorReport};

use crate::event_loop::future::FutureQueue;
use crate::event_loop::macrotasks::MacrotaskQueue;
use crate::event_loop::microtasks::MicrotaskQueue;

pub(crate) mod future;
pub(crate) mod macrotasks;
pub(crate) mod microtasks;

thread_local! {
	pub(crate) static EVENT_LOOP: RefCell<EventLoop> = RefCell::new(
		EventLoop {
			futures: None,
			microtasks: None,
			macrotasks: None,
			waker: AtomicWaker::new(),
		}
	);
}

pub struct EventLoop {
	pub(crate) futures: Option<Rc<FutureQueue>>,
	pub(crate) microtasks: Option<Rc<MicrotaskQueue>>,
	pub(crate) macrotasks: Option<Rc<MacrotaskQueue>>,
	pub(crate) waker: AtomicWaker,
}

impl EventLoop {
	pub async fn run_event_loop(&self, cx: Context) -> Result<(), Option<ErrorReport>> {
		poll_fn(|wcx| self.poll_event_loop(cx, wcx)).await
	}

	fn poll_event_loop(&self, cx: Context, wcx: &mut task::Context) -> Poll<Result<(), Option<ErrorReport>>> {
		{
			self.waker.register(wcx.waker());
		}

		if let Some(ref futures) = self.futures {
			if !futures.is_empty() {
				futures.run_futures(cx, wcx)?;
			}
		}

		if let Some(ref microtasks) = self.microtasks {
			if !microtasks.is_empty() {
				microtasks.run_jobs(cx)?;
			}
		}

		if let Some(ref macrotasks) = self.macrotasks {
			if !macrotasks.is_empty() {
				macrotasks.run_all(cx)?;
			}
		}

		if self.is_empty() {
			Poll::Ready(Ok(()))
		} else {
			self.waker.wake();
			Poll::Pending
		}
	}

	fn is_empty(&self) -> bool {
		self.microtasks.as_ref().map(|m| m.is_empty()).unwrap_or(true)
			&& self.futures.as_ref().map(|f| f.is_empty()).unwrap_or(true)
			&& self.macrotasks.as_ref().map(|m| m.is_empty()).unwrap_or(true)
	}
}
