/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::future::Future;
use std::mem::transmute;
use std::pin::Pin;
use std::task;
use std::task::Poll;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use mozjs::jsapi::JSFunction;

use ion::{Context, ErrorReport, Function, Object, Value};
use ion::conversions::BoxedIntoValue;

pub type NativeFuture =
	Pin<Box<dyn Future<Output = (*mut JSFunction, *mut JSFunction, Result<BoxedIntoValue<'static>, BoxedIntoValue<'static>>)> + 'static>>;

#[derive(Default)]
pub struct FutureQueue {
	queue: RefCell<FuturesUnordered<NativeFuture>>,
}

impl FutureQueue {
	pub fn run_futures<'cx>(&self, cx: &'cx Context, wcx: &mut task::Context) -> Result<(), Option<ErrorReport>> {
		let mut results: Vec<(*mut JSFunction, *mut JSFunction, Result<BoxedIntoValue, BoxedIntoValue>)> = Vec::new();

		let mut queue = self.queue.borrow_mut();
		while let Poll::Ready(Some(item)) = queue.poll_next_unpin(wcx) {
			results.push(item);
		}

		for (resolve, reject, result) in results {
			let null = Object::null(cx);
			let mut value = Value::undefined(cx);

			unsafe {
				match result {
					Ok(o) => {
						let o: BoxedIntoValue<'cx> = transmute(o);
						o.into_value(cx, &mut value);
						let resolve = Function::from(cx.root_function(resolve));
						resolve.call(cx, &null, &[value])?;
					}
					Err(e) => {
						let e: BoxedIntoValue<'cx> = transmute(e);
						e.into_value(cx, &mut value);
						let reject = Function::from(cx.root_function(reject));
						reject.call(cx, &null, &[value])?;
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
