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

use ion::{Context, ErrorReport};

use crate::event_loop::future::FutureQueue;
use crate::event_loop::macrotasks::MacrotaskQueue;
use crate::event_loop::microtasks::MicrotaskQueue;

pub(crate) mod future;
pub(crate) mod macrotasks;
pub(crate) mod microtasks;

thread_local!(pub(crate) static EVENT_LOOP: RefCell<EventLoop> = RefCell::new(EventLoop::default()));

#[derive(Default)]
pub struct EventLoop {
	pub(crate) futures: Option<Rc<FutureQueue>>,
	pub(crate) microtasks: Option<Rc<MicrotaskQueue>>,
	pub(crate) macrotasks: Option<Rc<MacrotaskQueue>>,
}

impl EventLoop {
	pub async fn run_event_loop(&self, cx: &Context) -> Result<(), Option<ErrorReport>> {
		let mut complete = false;
		poll_fn(|wcx| self.poll_event_loop(cx, wcx, &mut complete)).await
	}

	fn poll_event_loop(&self, cx: &Context, wcx: &mut task::Context, complete: &mut bool) -> Poll<Result<(), Option<ErrorReport>>> {
		if let Some(futures) = &self.futures {
			if !futures.is_empty() {
				futures.run_futures(cx, wcx)?;
			}
		}

		if let Some(microtasks) = &self.microtasks {
			if !microtasks.is_empty() {
				microtasks.run_jobs(cx)?;
			}
		}

		if let Some(macrotasks) = &self.macrotasks {
			if !macrotasks.is_empty() {
				macrotasks.run_jobs(cx)?;
			}
		}

		let empty = self.is_empty();
		if empty && *complete {
			Poll::Ready(Ok(()))
		} else {
			wcx.waker().wake_by_ref();
			*complete = empty;
			Poll::Pending
		}
	}

	fn is_empty(&self) -> bool {
		self.microtasks.as_ref().map(|m| m.is_empty()).unwrap_or(true)
			&& self.futures.as_ref().map(|f| f.is_empty()).unwrap_or(true)
			&& self.macrotasks.as_ref().map(|m| m.is_empty()).unwrap_or(true)
	}
}
