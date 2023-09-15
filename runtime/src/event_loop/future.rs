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
use mozjs::jsapi::{JS_GetFunctionObject, JSFunction};

use ion::{Context, ErrorReport, Function, Object, Value};
use ion::conversions::BoxedIntoValue;

type FutureOutput = (*mut JSFunction, *mut JSFunction, Result<BoxedIntoValue, BoxedIntoValue>);
pub type NativeFuture = Pin<Box<dyn Future<Output = FutureOutput> + 'static>>;

#[derive(Default)]
pub struct FutureQueue {
	queue: RefCell<FuturesUnordered<NativeFuture>>,
}

impl FutureQueue {
	pub fn run_futures(&self, cx: &Context, wcx: &mut task::Context) -> Result<(), Option<ErrorReport>> {
		let mut results = Vec::new();

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
						o.into_value(cx, &mut value);
						let resolve = Function::from(cx.root_function(resolve));
						resolve.call(cx, &null, &[value])?;
					}
					Err(e) => {
						e.into_value(cx, &mut value);
						let reject = Function::from(cx.root_function(reject));
						reject.call(cx, &null, &[value])?;
					}
				}

				cx.unroot_persistent_object(JS_GetFunctionObject(resolve));
				cx.unroot_persistent_object(JS_GetFunctionObject(reject));
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
