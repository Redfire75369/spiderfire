/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::future::Future;

use tokio::task::spawn_local;

use ion::{Context, Promise};
use ion::conversions::{BoxedIntoValue, IntoValue};

use crate::ContextExt;

/// Returns None if no future queue has been initialised.
pub fn future_to_promise<'cx, F, O, E>(cx: &'cx Context, future: F) -> Option<Promise<'cx>>
where
	F: Future<Output = Result<O, E>> + 'static,
	O: for<'cx2> IntoValue<'cx2> + 'static,
	E: for<'cx2> IntoValue<'cx2> + 'static,
{
	let promise = Promise::new(cx);
	let object = promise.handle().get();

	let handle = spawn_local(async move {
		let result: Result<BoxedIntoValue, BoxedIntoValue> = match future.await {
			Ok(o) => Ok(Box::new(o)),
			Err(e) => Err(Box::new(e)),
		};
		(result, object)
	});

	let event_loop = unsafe { &(*cx.get_private().as_ptr()).event_loop };
	event_loop.futures.as_ref().map(|futures| {
		futures.enqueue(handle);
		promise
	})
}
