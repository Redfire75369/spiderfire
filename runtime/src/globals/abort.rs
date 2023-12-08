/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{ptr, task};
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;

use chrono::Duration;
use mozjs::jsapi::JSObject;
use mozjs::jsval::JSVal;
use tokio::sync::watch::{channel, Receiver, Sender};

use ion::{ClassDefinition, Context, Error, ErrorKind, Exception, Object, Result, ResultExc, Value};
use ion::class::Reflector;
use ion::conversions::{ConversionBehavior, FromValue, ToValue};

use crate::ContextExt;
use crate::event_loop::macrotasks::{Macrotask, SignalMacrotask};

#[derive(Clone, Debug, Default)]
pub enum Signal {
	#[default]
	None,
	Abort(JSVal),
	Receiver(Receiver<Option<JSVal>>),
	Timeout(Receiver<Option<JSVal>>, Arc<AtomicBool>),
}

impl Signal {
	pub fn poll(&self) -> SignalFuture {
		SignalFuture { inner: self.clone() }
	}
}

pub struct SignalFuture {
	inner: Signal,
}

impl Future for SignalFuture {
	type Output = JSVal;

	fn poll(mut self: Pin<&mut SignalFuture>, cx: &mut task::Context) -> Poll<JSVal> {
		match &mut self.inner {
			Signal::None => Poll::Pending,
			Signal::Abort(abort) => Poll::Ready(*abort),
			Signal::Receiver(receiver) | Signal::Timeout(receiver, _) => {
				if let Some(abort) = *receiver.borrow() {
					return Poll::Ready(abort);
				}
				let changed = { pin!(receiver.changed()).poll(cx) };
				match changed {
					Poll::Ready(_) => match *receiver.borrow() {
						Some(abort) => Poll::Ready(abort),
						None => Poll::Pending,
					},
					Poll::Pending => Poll::Pending,
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
pub struct AbortController {
	reflector: Reflector,
	#[ion(no_trace)]
	sender: Sender<Option<JSVal>>,
}

#[js_class]
impl AbortController {
	#[ion(constructor)]
	pub fn constructor() -> AbortController {
		let (sender, _) = channel(None);
		AbortController { reflector: Reflector::default(), sender }
	}

	// TODO: Return the same signal object
	#[ion(get)]
	pub fn get_signal(&self, cx: &Context) -> *mut JSObject {
		AbortSignal::new_object(
			cx,
			Box::new(AbortSignal {
				reflector: Reflector::default(),
				signal: Signal::Receiver(self.sender.subscribe()),
			}),
		)
	}

	pub fn abort<'cx>(&self, cx: &'cx Context, reason: Option<Value<'cx>>) {
		let reason = reason.unwrap_or_else(|| Error::new("AbortError", None).as_value(cx));
		self.sender.send_replace(Some(reason.get()));
	}
}

#[js_class]
#[derive(Default)]
pub struct AbortSignal {
	reflector: Reflector,
	#[ion(no_trace)]
	pub(crate) signal: Signal,
}

#[js_class]
impl AbortSignal {
	#[ion(constructor)]
	pub fn constructor() -> Result<AbortSignal> {
		Err(Error::new("AbortSignal has no constructor.", ErrorKind::Type))
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

	#[ion(name = "throwIfAborted")]
	pub fn throw_if_aborted(&self) -> ResultExc<()> {
		if let Some(reason) = self.get_reason() {
			Err(Exception::Other(reason))
		} else {
			Ok(())
		}
	}

	pub fn abort<'cx>(cx: &'cx Context, reason: Option<Value<'cx>>) -> *mut JSObject {
		let reason = reason.unwrap_or_else(|| Error::new("AbortError", None).as_value(cx));
		AbortSignal::new_object(
			cx,
			Box::new(AbortSignal {
				reflector: Reflector::default(),
				signal: Signal::Abort(reason.get()),
			}),
		)
	}

	pub fn timeout(cx: &Context, #[ion(convert = ConversionBehavior::EnforceRange)] time: u64) -> *mut JSObject {
		let (sender, receiver) = channel(None);
		let terminate = Arc::new(AtomicBool::new(false));
		let terminate2 = terminate.clone();

		let error = Error::new(&format!("Timeout Error: {}ms", time), None).as_value(cx).get();
		let callback = Box::new(move || {
			sender.send_replace(Some(error));
		});

		let duration = Duration::milliseconds(time as i64);
		let event_loop = unsafe { &mut cx.get_private().event_loop };
		if let Some(queue) = &mut event_loop.macrotasks {
			queue.enqueue(
				Macrotask::Signal(SignalMacrotask::new(callback, terminate, duration)),
				None,
			);
			AbortSignal::new_object(
				cx,
				Box::new(AbortSignal {
					reflector: Reflector::default(),
					signal: Signal::Timeout(receiver, terminate2),
				}),
			)
		} else {
			ptr::null_mut()
		}
	}
}

impl<'cx> FromValue<'cx> for AbortSignal {
	type Config = ();
	fn from_value(cx: &'cx Context, value: &Value, strict: bool, _: ()) -> Result<AbortSignal> {
		let object = Object::from_value(cx, value, strict, ())?;
		if AbortSignal::instance_of(cx, &object, None) {
			Ok(AbortSignal {
				reflector: Reflector::default(),
				signal: AbortSignal::get_private(&object).signal.clone(),
			})
		} else {
			Err(Error::new("Expected AbortSignal", ErrorKind::Type))
		}
	}
}

pub fn define(cx: &Context, global: &mut Object) -> bool {
	AbortController::init_class(cx, global).0 && AbortSignal::init_class(cx, global).0
}
