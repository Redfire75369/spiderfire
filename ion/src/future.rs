/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::future::Future;
use std::pin::Pin;
use std::task;
use std::task::Poll;

use futures::channel::mpsc;
use futures::channel::mpsc::Receiver;
use futures::Stream;
use mozjs::jsval::JSVal;

use crate::{Context, Function, Promise, Value};
use crate::flags::PropertyFlags;

pub struct PromiseFuture(Receiver<Result<JSVal, JSVal>>);

impl PromiseFuture {
	pub fn new(cx: &Context, promise: &Promise) -> PromiseFuture {
		let (rx, tx) = mpsc::channel(1);

		let mut rx1 = rx;
		let mut rx2 = rx1.clone();

		promise.add_reactions(
			cx,
			Some(Function::from_closure(
				cx,
				"",
				Box::new(move |args| {
					let _ = rx1.try_send(Ok(args.value(0).unwrap().get()));
					Ok(Value::undefined(args.cx()))
				}),
				1,
				PropertyFlags::empty(),
			)),
			Some(Function::from_closure(
				cx,
				"",
				Box::new(move |args| {
					let _ = rx2.try_send(Err(args.value(0).unwrap().get()));
					Ok(Value::undefined(args.cx()))
				}),
				1,
				PropertyFlags::empty(),
			)),
		);

		PromiseFuture(tx)
	}
}

impl Future for PromiseFuture {
	type Output = Result<JSVal, JSVal>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Result<JSVal, JSVal>> {
		let result = Pin::new(&mut self.0);
		if let Poll::Ready(Some(val)) = result.poll_next(cx) {
			Poll::Ready(val)
		} else {
			Poll::Pending
		}
	}
}
