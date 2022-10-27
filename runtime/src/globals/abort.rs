/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;

use futures::FutureExt;
use mozjs::jsval::JSVal;
use tokio::sync::watch::Receiver;

pub use controller::AbortController;
use ion::{ClassInitialiser, Context, Object};
pub use signal::AbortSignal;

#[derive(Clone, Debug)]
pub enum Signal {
	None,
	Abort(JSVal),
	Receiver(Receiver<Option<JSVal>>),
	Timeout(Receiver<Option<JSVal>>, Arc<AtomicBool>),
}

impl Default for Signal {
	fn default() -> Self {
		Signal::None
	}
}

pub struct SignalFuture {
	inner: Signal,
}

impl Future for SignalFuture {
	type Output = JSVal;

	fn poll(mut self: Pin<&mut SignalFuture>, cx: &mut std::task::Context<'_>) -> Poll<JSVal> {
		match &mut self.inner {
			Signal::None => Poll::Pending,
			Signal::Abort(abort) => Poll::Ready(*abort),
			Signal::Receiver(receiver) | Signal::Timeout(receiver, _) => {
				if let Some(abort) = *receiver.borrow() {
					return Poll::Ready(abort);
				}
				let changed = { Box::pin(receiver.changed()).poll_unpin(cx) };
				match changed {
					Poll::Ready(_) => match *receiver.borrow() {
						Some(abort) => Poll::Ready(abort),
						None => {
							cx.waker().wake_by_ref();
							Poll::Pending
						}
					},
					Poll::Pending => {
						cx.waker().wake_by_ref();
						Poll::Pending
					}
				}
			}
		}
	}
}

impl Drop for SignalFuture {
	fn drop(&mut self) {
		if let Signal::Timeout(receiver, terminate) = &self.inner {
			if receiver.borrow().is_none() {
				terminate.store(true, Ordering::SeqCst);
			}
		}
	}
}

#[js_class]
mod controller {
	use mozjs::jsval::JSVal;
	use tokio::sync::watch::{channel, Sender};

	use ion::{Context, Error, Value};
	use ion::conversions::ToValue;

	use crate::globals::abort::{AbortSignal, Signal};

	#[ion(into_value)]
	pub struct AbortController {
		sender: Sender<Option<JSVal>>,
	}

	impl AbortController {
		#[ion(constructor)]
		pub fn constructor() -> AbortController {
			let (sender, _) = channel(None);
			AbortController { sender }
		}

		#[ion(get)]
		pub fn get_signal(&self) -> AbortSignal {
			AbortSignal {
				signal: Signal::Receiver(self.sender.subscribe()),
			}
		}

		pub fn abort<'cx>(&self, cx: &'cx Context, reason: Option<Value<'cx>>) {
			let none = reason.is_none();
			let mut reason = reason.unwrap_or_else(|| Value::undefined(cx));
			if none {
				unsafe {
					Error::new("AbortError", None).to_value(cx, &mut reason);
				}
			}
			self.sender.send_replace(Some(**reason));
		}
	}
}

#[js_class]
mod signal {
	use std::result;
	use std::sync::Arc;
	use std::sync::atomic::AtomicBool;

	use chrono::Duration;
	use mozjs::jsval::JSVal;
	use tokio::sync::watch::channel;

	use ion::{Context, Error, Exception, Value};
	use ion::conversions::{ConversionBehavior, ToValue};

	use crate::event_loop::EVENT_LOOP;
	use crate::event_loop::macrotasks::{Macrotask, SignalMacrotask};
	use crate::globals::abort::{Signal, SignalFuture};

	#[derive(Clone, Default)]
	#[ion(no_constructor, from_value, to_value)]
	pub struct AbortSignal {
		pub(crate) signal: Signal,
	}

	impl AbortSignal {
		#[ion(skip)]
		pub fn poll(&self) -> SignalFuture {
			SignalFuture { inner: self.signal.clone() }
		}

		#[ion(get)]
		pub fn get_aborted(&self) -> bool {
			self.get_reason().is_some()
		}

		#[ion(get)]
		pub fn get_reason(&self) -> Option<JSVal> {
			match &self.signal {
				Signal::None => None,
				Signal::Abort(abort) => Some(*abort),
				Signal::Receiver(receiver) | Signal::Timeout(receiver, _) => *receiver.borrow(),
			}
		}

		pub fn throwIfAborted(&self) -> result::Result<(), Exception> {
			if let Some(reason) = self.get_reason() {
				Err(Exception::Other(reason))
			} else {
				Ok(())
			}
		}

		pub fn abort<'cx>(cx: &'cx Context, reason: Option<Value<'cx>>) -> AbortSignal {
			let none = reason.is_none();
			let mut reason = reason.unwrap_or_else(|| Value::undefined(cx));
			if none {
				unsafe {
					Error::new("AbortError", None).to_value(cx, &mut reason);
				}
			}
			AbortSignal { signal: Signal::Abort(**reason) }
		}

		pub fn timeout(cx: &Context, #[ion(convert = ConversionBehavior::EnforceRange)] time: u64) -> AbortSignal {
			let (sender, receiver) = channel(None);
			let terminate = Arc::new(AtomicBool::new(false));
			let terminate2 = terminate.clone();

			let mut error = Value::null(cx);
			unsafe {
				Error::new(&format!("Timeout Error: {}ms", time), None).to_value(cx, &mut error);
			}
			let error = **error;
			let callback = Box::new(move || {
				sender.send_replace(Some(error));
			});

			let duration = Duration::milliseconds(time as i64);
			EVENT_LOOP.with(|event_loop| {
				if let Some(queue) = (*event_loop.borrow_mut()).macrotasks.as_mut() {
					queue.enqueue(Macrotask::Signal(SignalMacrotask::new(callback, terminate, duration)), None);
				}
			});
			AbortSignal {
				signal: Signal::Timeout(receiver, terminate2),
			}
		}
	}
}

pub fn define(cx: &Context, global: &mut Object) -> bool {
	AbortController::init_class(cx, global);
	AbortSignal::init_class(cx, global);
	true
}
