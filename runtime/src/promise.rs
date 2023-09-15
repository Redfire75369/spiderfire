/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::future::Future;

use futures::channel::oneshot::channel;
use mozjs::jsapi::JSFunction;

use ion::{Context, Error, Promise};
use ion::conversions::{BoxedIntoValue, IntoValue};
use ion::utils::SendWrapper;

use crate::event_loop::EVENT_LOOP;
use crate::event_loop::future::NativeFuture;

pub fn future_to_promise<'cx, F, O, E>(cx: &'cx Context, future: F) -> Option<Promise<'cx>>
where
	F: Future<Output = Result<O, E>> + 'static + Send,
	O: for<'cx2> IntoValue<'cx2> + 'static,
	E: for<'cx2> IntoValue<'cx2> + 'static,
{
	let (tx, rx) = channel::<(SendWrapper<*mut JSFunction>, SendWrapper<*mut JSFunction>)>();

	let future = async move {
		let (resolve, reject) = rx.await.unwrap();
		let result = future.await;

		let result: Result<BoxedIntoValue, BoxedIntoValue> = match result {
			Ok(o) => Ok(Box::new(o)),
			Err(e) => Err(Box::new(e)),
		};
		(resolve.take(), reject.take(), result)
	};

	let future: NativeFuture = Box::pin(future);
	EVENT_LOOP.with(move |event_loop| {
		let event_loop = event_loop.borrow_mut();
		if let Some(ref futures) = event_loop.futures {
			futures.enqueue(future);
		}
	});

	Promise::new_with_executor(cx, move |_, resolve, reject| {
		tx.send((SendWrapper::new(resolve.get()), SendWrapper::new(reject.get())))
			.map_err(|_| Error::new("Failed to send resolve and reject through channel", None))
	})
}
