/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task;
use std::task::Poll;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use mozjs::rust::HandleObject;

use ion::{Context, ErrorReport, Function, Object, Value};
use ion::conversions::BoxedIntoJSVal;

pub type NativeFuture = Pin<Box<dyn Future<Output = (Function, Function, Result<BoxedIntoJSVal, BoxedIntoJSVal>)> + 'static>>;

#[derive(Default)]
pub struct FutureQueue {
	queue: RefCell<FuturesUnordered<NativeFuture>>,
}

impl FutureQueue {
	pub fn run_futures(&self, cx: Context, wcx: &mut task::Context) -> Result<(), Option<ErrorReport>> {
		let mut results: Vec<(Function, Function, Result<BoxedIntoJSVal, BoxedIntoJSVal>)> = Vec::new();

		let mut queue = self.queue.borrow_mut();
		while let Poll::Ready(Some(item)) = queue.poll_next_unpin(wcx) {
			results.push(item);
		}

		for (resolve, reject, result) in results {
			let null = Object::from(HandleObject::null().get());
			rooted!(in(cx) let mut value = *Value::undefined());

			unsafe {
				match result {
					Ok(o) => {
						o.into_jsval(cx, value.handle_mut());
						resolve.call(cx, null, vec![value.get()])?;
					}
					Err(e) => {
						e.into_jsval(cx, value.handle_mut());
						reject.call(cx, null, vec![value.get()])?;
					}
				}
			}
		}

		Ok(())
	}

	pub fn enqueue(&self, fut: NativeFuture) {
		self.queue.borrow().push(fut);
	}

	pub fn is_empty(&self) -> bool {
		self.queue.borrow().is_empty()
	}
}
