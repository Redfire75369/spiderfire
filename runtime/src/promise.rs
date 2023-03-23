/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::future::Future;
use std::mem::transmute;

use futures::channel::oneshot::channel;
use mozjs::jsapi::JSFunction;

use ion::{Context, Error, Promise};
use ion::conversions::{BoxedIntoValue, IntoValue};

use crate::event_loop::EVENT_LOOP;
use crate::event_loop::future::NativeFuture;

pub fn future_to_promise<'cx, F, O, E>(cx: &'cx Context, future: F) -> Option<Promise<'cx>>
where
	F: Future<Output = Result<O, E>> + 'static + Send,
	O: IntoValue<'cx> + 'static,
	E: IntoValue<'cx> + 'static,
{
	let (tx, rx) = channel::<(UnsafeAssertSend<*mut JSFunction>, UnsafeAssertSend<*mut JSFunction>)>();

	let future = async move {
		let (resolve, reject) = rx.await.unwrap();
		let result = future.await;

		let result: Result<BoxedIntoValue, BoxedIntoValue> = match result {
			Ok(o) => Ok(Box::new(o)),
			Err(e) => Err(Box::new(e)),
		};
		(resolve.into_inner(), reject.into_inner(), unsafe { transmute(result) })
	};

	let future: NativeFuture = Box::pin(future);
	EVENT_LOOP.with(move |event_loop| {
		let event_loop = event_loop.borrow_mut();
		if let Some(ref futures) = event_loop.futures {
			futures.enqueue(future);
		}
	});

	Promise::new_with_executor(cx, move |_, resolve, reject| {
		tx.send(unsafe { (UnsafeAssertSend::new(**resolve), UnsafeAssertSend::new(**reject)) })
			.map_err(|_| Error::new("Failed to send resolve and reject through channel", None))
	})
}

pub struct UnsafeAssertSend<T>(T);

impl<T> UnsafeAssertSend<T> {
	/// ### Safety
	/// This instance must be used in a thread-safe way.
	pub unsafe fn new(value: T) -> Self {
		Self(value)
	}
	pub fn into_inner(self) -> T {
		self.0
	}
}

/// ### Safety
/// See [UnsafeAssertSend::new]
unsafe impl<T> Send for UnsafeAssertSend<T> {}
