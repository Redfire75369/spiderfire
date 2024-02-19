/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::Cell;
use std::rc::Rc;

use bytes::{Buf, Bytes};
use mozjs::gc::HandleObject;
use mozjs::jsapi::{Heap, JSFunction, JSObject};
use mozjs::jsval::{JSVal, UndefinedValue};

use ion::{ClassDefinition, Context, Function, Local, Object, Promise, ResultExc, TracedHeap, Value};
use ion::class::NativeObject;
use ion::conversions::{FromValue, ToValue};
use ion::function::Opt;
use ion::typedarray::ArrayBuffer;

use crate::globals::streams::readable::controller::Controller;
use crate::globals::streams::readable::ReadableStream;
use crate::globals::streams::readable::reader::{Reader, Request};

#[derive(Traceable)]
pub enum StreamSource {
	None,
	Script {
		object: Box<Heap<*mut JSObject>>,
		pull: Option<Box<Heap<*mut JSFunction>>>,
		cancel: Option<Box<Heap<*mut JSFunction>>>,
	},
	Bytes(#[trace(no_trace)] Option<Bytes>),
	BytesBuf(#[trace(no_trace)] Option<Box<dyn Buf>>),
	Tee(Rc<TeeState>, bool),
}

impl StreamSource {
	pub fn source_object(&self) -> Object {
		match self {
			StreamSource::Script { object, .. } => Object::from(unsafe { Local::from_heap(object) }),
			_ => Object::from(Local::from_handle(HandleObject::null())),
		}
	}

	pub fn pull<'cx>(&mut self, cx: &'cx Context, controller: *mut JSObject) -> ResultExc<Option<Promise<'cx>>> {
		match self {
			StreamSource::Script { object, pull: Some(pull), .. } => {
				let pull = Function::from(unsafe { Local::from_heap(pull) });
				let controller = controller.as_value(cx);
				let this = Object::from(unsafe { Local::from_heap(object) });

				let result = pull.call(cx, &this, &[controller]).map_err(|report| report.unwrap().exception)?;
				Ok(Some(
					Promise::from_value(cx, &result, true, ()).unwrap_or_else(|_| Promise::new(cx)),
				))
			}
			StreamSource::Script { pull: None, .. } => Ok(Some(Promise::resolved(cx, &Value::undefined_handle()))),
			StreamSource::Bytes(bytes) => Ok(bytes.take().map(|bytes| {
				let buffer = ArrayBuffer::copy_from_bytes(cx, &bytes).unwrap();
				Promise::resolved(cx, &buffer.as_value(cx))
			})),
			StreamSource::BytesBuf(Some(buf)) => {
				if !buf.has_remaining() {
					return Ok(None);
				}

				let chunk = buf.chunk();
				let buffer = ArrayBuffer::copy_from_bytes(cx, chunk).unwrap();
				buf.advance(chunk.len());
				Ok(Some(Promise::resolved(cx, &buffer.as_value(cx))))
			}
			StreamSource::Tee(state, second) => {
				if state.reading.get() {
					state.read_again.set(true);
					return Ok(Some(Promise::resolved(cx, &Value::undefined_handle())));
				}

				state.reading.set(true);

				let state1 = Rc::clone(state);
				let state2 = Rc::clone(state);
				let state3 = Rc::clone(state);
				let second = *second;

				let promise = Promise::new(cx);
				let request = Request {
					promise: Heap::boxed(promise.get()),
					chunk: Box::new(move |cx, _, chunk| {
						let promise = Promise::resolved(cx, &Value::undefined_handle());
						let chunk = TracedHeap::new(chunk.get());
						let state = Rc::clone(&state1);

						promise.then(cx, move |cx, _| {
							state.read_again.set(false);
							let chunk = Value::from(chunk.to_local());

							// TODO: CloneForBranch2

							if !state.cancelled[0].get() {
								let branch = state.branch(cx, false)?;
								if let Controller::Default(controller) = branch.native_controller(cx)? {
									controller.enqueue_internal(cx, &chunk)?;
								}
							}
							if !state.cancelled[1].get() {
								let branch = state.branch(cx, true)?;
								if let Controller::Default(controller) = branch.native_controller(cx)? {
									controller.enqueue_internal(cx, &chunk)?;
								}
							}

							state.reading.set(false);
							if state.read_again.get() {
								let branch = state.branch(cx, second)?;
								if let Controller::Default(controller) = branch.native_controller(cx)? {
									controller.common.source.pull(cx, controller.reflector().get())?;
								}
							}
							Ok(Value::undefined_handle())
						});
					}),
					close: Box::new(move |cx, _, _| {
						state2.reading.set(false);

						if !state2.cancelled[0].get() {
							let branch = state2.branch(cx, false)?;
							if let Controller::Default(controller) = branch.native_controller(cx)? {
								controller.close(cx)?;
							}
						}
						if !state2.cancelled[1].get() {
							let branch = state2.branch(cx, true)?;
							if let Controller::Default(controller) = branch.native_controller(cx)? {
								controller.close(cx)?;
							}
						}

						if !state2.cancelled[0].get() || !state2.cancelled[1].get() {
							let promise = Promise::from(unsafe { Local::from_heap(&state2.cancel_promise) }).unwrap();
							promise.resolve(cx, &Value::undefined_handle());
						}

						Ok(())
					}),
					error: Box::new(move |_, _, _| {
						state3.reading.set(false);
					}),
				};

				if let Some(Reader::Default(reader)) = state.stream(cx)?.native_reader(cx)? {
					reader.read_internal(cx, request)?;
				}

				promise.resolve(cx, &Value::undefined_handle());
				Ok(Some(promise))
			}
			_ => Ok(None),
		}
	}

	pub fn cancel(&mut self, cx: &Context, promise: &mut Promise, reason: Option<Value>) -> ResultExc<()> {
		match self {
			StreamSource::Script { object, cancel: Some(cancel), .. } => {
				let cancel = Function::from(unsafe { Local::from_heap(cancel) });
				let this = Object::from(unsafe { Local::from_heap(object) });
				let reason = reason.unwrap_or_else(Value::undefined_handle);

				let result = cancel.call(cx, &this, &[reason]).map_err(|report| report.unwrap().exception)?;
				if let Ok(result) = Promise::from_value(cx, &result, true, ()) {
					result.then(cx, |_, _| Ok(Value::undefined_handle()));
					promise.handle_mut().set(result.get());
				} else {
					promise.resolve(cx, &Value::undefined_handle());
				}
			}
			StreamSource::Tee(state, second) => {
				let reason = reason.unwrap_or_else(Value::undefined_handle);

				let branch = usize::from(*second);
				state.cancelled[branch].set(true);
				state.reason[branch].set(reason.get());

				if state.cancelled[usize::from(!*second)].get() {
					let composite = [state.reason[0].get(), state.reason[1].get()].as_value(cx);
					let result = state.stream(cx)?.cancel(cx, Opt(Some(composite)))?;

					let cancel_promise = Promise::from(unsafe { Local::from_heap(&state.cancel_promise) }).unwrap();
					cancel_promise.resolve(cx, &result.as_value(cx));
				}

				promise.handle_mut().set(state.cancel_promise.get());
			}
			_ => {}
		}

		Ok(())
	}

	pub fn clear_algorithms(&mut self) {
		match self {
			StreamSource::Script { pull, cancel, .. } => {
				*pull = None;
				*cancel = None;
			}
			StreamSource::Bytes(bytes) => {
				*bytes = None;
			}
			StreamSource::BytesBuf(buf) => {
				*buf = None;
			}
			_ => {}
		}
	}
}

#[derive(Traceable)]
pub struct TeeState {
	pub(crate) reading: Cell<bool>,
	pub(crate) read_again: Cell<bool>,
	pub(crate) cancelled: [Cell<bool>; 2],

	pub(crate) clone_branch_2: bool,
	pub(crate) stream: Box<Heap<*mut JSObject>>,

	pub(crate) branch: [Box<Heap<*mut JSObject>>; 2],
	pub(crate) reason: [Box<Heap<JSVal>>; 2],
	pub(crate) cancel_promise: Box<Heap<*mut JSObject>>,
}

impl TeeState {
	pub(crate) fn new(cx: &Context, stream: &ReadableStream, clone_branch_2: bool) -> TeeState {
		let promise = Promise::new(cx);
		TeeState {
			reading: Cell::new(false),
			read_again: Cell::new(false),
			cancelled: [Cell::new(false), Cell::new(false)],

			clone_branch_2,
			stream: Heap::boxed(stream.reflector.get()),

			branch: [Box::default(), Box::default()],
			reason: [Heap::boxed(UndefinedValue()), Heap::boxed(UndefinedValue())],
			cancel_promise: Heap::boxed(promise.get()),
		}
	}

	pub(crate) fn stream(&self, cx: &Context) -> ion::Result<&mut ReadableStream> {
		let stream = Object::from(unsafe { Local::from_heap(&self.stream) });
		ReadableStream::get_mut_private(cx, &stream)
	}

	pub(crate) fn branch(&self, cx: &Context, second: bool) -> ion::Result<&mut ReadableStream> {
		let stream = Object::from(unsafe { Local::from_heap(&self.branch[usize::from(second)]) });
		ReadableStream::get_mut_private(cx, &stream)
	}
}
