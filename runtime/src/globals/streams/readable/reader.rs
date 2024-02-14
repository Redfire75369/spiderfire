/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::VecDeque;

use mozjs::jsapi::{Heap, JSObject};

use ion::{ClassDefinition, Context, Error, ErrorKind, Local, Object, Promise, Result, ResultExc, Value};
use ion::class::{Reflector};
use ion::conversions::ToValue;
use ion::function::Opt;
use ion::typedarray::{ArrayBufferView, type_to_constructor, type_to_element_size};

use crate::globals::streams::readable::{ReadableStream, State};
use crate::globals::streams::readable::controller::{Controller, ControllerKind, ControllerInternals, PullIntoDescriptor};

pub type ChunkErrorClosure = dyn Fn(&Context, &Promise, &Value);
pub type CloseClosure = dyn Fn(&Context, &Promise, Option<&Value>);

#[derive(Traceable)]
pub struct Request {
	promise: Box<Heap<*mut JSObject>>,
	#[trace(no_trace)]
	pub(crate) chunk: Box<ChunkErrorClosure>,
	#[trace(no_trace)]
	pub(crate) close: Box<CloseClosure>,
	#[trace(no_trace)]
	pub(crate) error: Box<ChunkErrorClosure>,
}

impl Request {
	pub(crate) fn standard(promise: *mut JSObject) -> Request {
		struct ReadResult<'cx> {
			pub value: Option<Value<'cx>>,
			pub done: bool,
		}

		fn into_value<'cx>(result: ReadResult, cx: &'cx Context) -> Value<'cx> {
			let object = Object::new(cx);
			object.set(cx, "value", &result.value.unwrap_or_else(Value::undefined_handle));
			object.set_as(cx, "done", &result.done);
			object.as_value(cx)
		}

		Request {
			promise: Heap::boxed(promise),
			chunk: Box::new(|cx, promise, value| {
				let result = ReadResult {
					value: Some(Value::from(Local::from_handle(value.handle()))),
					done: false,
				};
				promise.resolve(cx, &into_value(result, cx));
			}),
			close: Box::new(|cx, promise, value| {
				let result = ReadResult {
					value: value.map(|v| Value::from(Local::from_handle(v.handle()))),
					done: true,
				};
				promise.resolve(cx, &into_value(result, cx));
			}),
			error: Box::new(|cx, promise, error| {
				promise.resolve(cx, error);
			}),
		}
	}

	pub(crate) fn promise(&self) -> Promise {
		Promise::from(unsafe { Local::from_heap(&self.promise) }).unwrap()
	}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Traceable)]
pub enum ReaderKind {
	None,
	Default,
	Byob,
}

pub enum Reader<'r> {
	Default(&'r mut DefaultReader),
	Byob(&'r mut ByobReader),
}

impl<'r> Reader<'r> {
	pub fn common(&self) -> &CommonReader {
		match self {
			Reader::Default(reader) => &reader.common,
			Reader::Byob(reader) => &reader.common,
		}
	}

	pub fn requests_closed(self) -> (&'r mut VecDeque<Request>, Promise<'r>) {
		let common = match self {
			Reader::Default(reader) => &mut reader.common,
			Reader::Byob(reader) => &mut reader.common,
		};
		unsafe {
			(
				&mut common.requests,
				Promise::from(Local::from_heap(&common.closed)).unwrap(),
			)
		}
	}

	pub fn is_empty(&self) -> bool {
		self.common().requests.is_empty()
	}
}

#[js_class]
pub(crate) struct CommonReader {
	reflector: Reflector,

	stream: Option<Box<Heap<*mut JSObject>>>,
	pub(crate) requests: VecDeque<Request>,
	pub(crate) closed: Box<Heap<*mut JSObject>>,
}

#[js_class]
impl CommonReader {
	#[ion(constructor)]
	pub fn constructor() -> Result<CommonReader> {
		unreachable!()
	}

	pub(crate) fn new(cx: &Context, stream: &ReadableStream, stream_object: &Object) -> CommonReader {
		let closed = Promise::new(cx);
		match stream.state {
			State::Readable => {}
			State::Closed => {
				closed.resolve(cx, &Value::undefined_handle());
			}
			State::Errored => {
				closed.reject(cx, &stream.stored_error());
			}
		}

		CommonReader {
			reflector: Reflector::default(),
			stream: Some(Heap::boxed(stream_object.handle().get())),
			requests: VecDeque::new(),
			closed: Heap::boxed(closed.get()),
		}
	}

	pub(crate) fn stream(&self, cx: &Context) -> Result<Option<&mut ReadableStream>> {
		self.stream
			.as_ref()
			.map::<Result<_>, _>(|stream| {
				let stream = Object::from(unsafe { Local::from_heap(stream) });
				ReadableStream::get_mut_private(cx, &stream)
			})
			.transpose()
	}

	pub(crate) fn closed(&self) -> Promise {
		Promise::from(unsafe { Local::from_heap(&self.closed) }).unwrap()
	}

	pub(crate) fn cancel<'cx>(&self, cx: &'cx Context, reason: Opt<Value>) -> ResultExc<Promise<'cx>> {
		if let Some(stream) = self.stream(cx)? {
			stream.cancel(cx, reason)
		} else {
			let promise = Promise::new(cx);
			promise.reject_with_error(cx, &Error::new("Reader has already been released.", ErrorKind::Type));
			Ok(promise)
		}
	}

	pub(crate) fn release_lock(&mut self, cx: &Context) -> Result<()> {
		if let Some(stream) = self.stream(cx)? {
			let mut closed = self.closed();
			match stream.state {
				State::Readable => {}
				_ => {
					self.closed.set(Promise::new(cx).get());
					closed = self.closed();
				}
			}
			closed.reject_with_error(cx, &Error::new("Released Reader", ErrorKind::Type));

			stream.reader_kind = ReaderKind::None;
			stream.reader = None;

			stream.native_controller(cx)?.release();

			while let Some(request) = self.requests.pop_front() {
				let promise = request.promise();
				(request.error)(
					cx,
					&promise,
					&Error::new("Reader has been released.", ErrorKind::Type).as_value(cx),
				);
			}
		} else {
			return Err(Error::new("Reader has already been released.", ErrorKind::Type));
		}
		self.stream = None;
		Ok(())
	}
}

#[js_class]
#[ion(name = "ReadableStreamDefaultReader")]
pub struct DefaultReader {
	pub(crate) common: CommonReader,
}

#[js_class]
impl DefaultReader {
	#[ion(constructor)]
	pub fn constructor(cx: &Context, #[ion(this)] this: &Object, stream_object: Object) -> Result<DefaultReader> {
		let reader = DefaultReader::new(cx, &stream_object)?;
		let stream = ReadableStream::get_mut_private(cx, &stream_object)?;
		stream.reader_kind = ReaderKind::Default;
		stream.reader = Some(Heap::boxed(this.handle().get()));

		Ok(reader)
	}

	pub(crate) fn new(cx: &Context, stream_object: &Object) -> Result<DefaultReader> {
		let stream = ReadableStream::get_private(cx, stream_object)?;
		if stream.get_locked() {
			return Err(Error::new(
				"Cannot create DefaultReader from locked stream.",
				ErrorKind::Type,
			));
		}

		Ok(DefaultReader {
			common: CommonReader::new(cx, stream, stream_object),
		})
	}

	pub fn cancel<'cx>(&self, cx: &'cx Context, reason: Opt<Value>) -> ResultExc<Promise<'cx>> {
		self.common.cancel(cx, reason)
	}

	pub fn read<'cx>(&mut self, cx: &'cx Context) -> ResultExc<Promise<'cx>> {
		let promise = Promise::new(cx);
		let request = Request::standard(promise.get());

		if let Some(stream) = self.common.stream(cx)? {
			stream.disturbed = true;

			match stream.state {
				State::Readable => stream.native_controller(cx)?.pull(cx, &promise, request)?,
				State::Closed => (request.close)(cx, &promise, None),
				State::Errored => (request.error)(cx, &promise, &stream.stored_error()),
			}
		} else {
			promise.reject_with_error(cx, &Error::new("Reader has already been released.", ErrorKind::Type));
		}
		Ok(promise)
	}

	#[ion(name = "releaseLock")]
	pub fn release_lock(&mut self, cx: &Context) -> Result<()> {
		self.common.release_lock(cx)
	}

	#[ion(get)]
	pub fn get_closed(&self) -> *mut JSObject {
		self.common.closed.get()
	}
}

#[js_class]
#[ion(name = "ReadableStreamBYOBReader")]
pub struct ByobReader {
	pub(crate) common: CommonReader,
}

#[js_class]
impl ByobReader {
	#[ion(constructor)]
	pub fn constructor(cx: &Context, #[ion(this)] this: &Object, stream_object: Object) -> Result<ByobReader> {
		let reader = ByobReader::new(cx, &stream_object)?;
		let stream = ReadableStream::get_mut_private(cx, &stream_object)?;
		stream.reader_kind = ReaderKind::Byob;
		stream.reader = Some(Heap::boxed(this.handle().get()));

		Ok(reader)
	}

	pub(crate) fn new(cx: &Context, stream_object: &Object) -> Result<ByobReader> {
		let stream = ReadableStream::get_private(cx, stream_object)?;
		if stream.get_locked() {
			return Err(Error::new(
				"Cannot create BYOBReader from locked stream.",
				ErrorKind::Type,
			));
		}

		if stream.controller_kind == ControllerKind::Default {
			return Err(Error::new(
				"Cannot create BYOBReader from DefaultController",
				ErrorKind::Type,
			));
		}

		Ok(ByobReader {
			common: CommonReader::new(cx, stream, stream_object),
		})
	}

	pub fn cancel<'cx>(&self, cx: &'cx Context, reason: Opt<Value>) -> ResultExc<Promise<'cx>> {
		self.common.cancel(cx, reason)
	}

	pub fn read<'cx>(&mut self, cx: &'cx Context, view: ArrayBufferView) -> ResultExc<Promise<'cx>> {
		let promise = Promise::new(cx);
		let request = Request::standard(promise.get());

		if let Some(stream) = self.common.stream(cx)? {
			if view.data().1 == 0 {
				return Err(Error::new("View must not be empty.", ErrorKind::Type).into());
			}

			let buffer = view.buffer(cx);

			if buffer.data().1 == 0 {
				return Err(Error::new("Buffer must contain bytes.", ErrorKind::Type).into());
			}

			if buffer.is_detached() {
				return Err(Error::new("ArrayBuffer must not be detached.", ErrorKind::Type).into());
			}

			stream.disturbed = true;
			if stream.state == State::Errored {
				(request.error)(cx, &promise, &stream.stored_error());
				return Ok(promise);
			}

			let (constructor, element_size) = {
				let ty = view.view_type();
				(type_to_constructor(ty), type_to_element_size(ty))
			};

			let offset = view.offset();
			let length = view.data().1;
			match buffer.transfer(cx) {
				Ok(buffer) => {
					let mut descriptor = PullIntoDescriptor {
						buffer: Heap::boxed(buffer.get()),
						offset,
						length: length * element_size,
						filled: 0,
						element: element_size,
						constructor,
						kind: ReaderKind::Byob,
					};

					if let Controller::ByteStream(controller) = stream.native_controller(cx)? {
						if !controller.pending_descriptors.is_empty() {
							controller.pending_descriptors.push_back(descriptor);

							if stream.state == State::Readable {
								self.common.requests.push_back(request);
							}
							return Ok(promise);
						} else if stream.state == State::Closed {
							let empty = descriptor.construct(cx)?.as_value(cx);
							(request.close)(cx, &promise, Some(&empty));
							return Ok(promise);
						} else if controller.common.queue_size > 0 {
							if controller.fill_pull_into_descriptor(cx, &mut descriptor)? {
								let buffer = buffer.transfer(cx)?;
								descriptor.buffer.set(buffer.get());
								let view = descriptor.construct(cx)?.as_value(cx);

								if controller.common.queue_size == 0 && controller.common.close_requested {
									controller.close(cx)?;
								} else {
									controller.pull_if_needed(cx)?;
								}

								(request.chunk)(cx, &promise, &view);
								return Ok(promise);
							} else if controller.common.close_requested {
								let error = Error::new("Stream closed by request.", ErrorKind::Type);
								// TODO: ByteStreamController Error
								(request.error)(cx, &promise, &error.as_value(cx));
								return Ok(promise);
							}
						}

						controller.pending_descriptors.push_back(descriptor);
						controller.pull_if_needed(cx)?;

						if stream.state == State::Readable {
							self.common.requests.push_back(request);
						}
					}
				}
				Err(error) => {
					(request.error)(cx, &promise, &error.as_value(cx));
				}
			}
		} else {
			(request.error)(
				cx,
				&promise,
				&Error::new("Reader has already been released.", ErrorKind::Type).as_value(cx),
			);
		}
		Ok(promise)
	}

	#[ion(name = "releaseLock")]
	pub fn release_lock(&mut self, cx: &Context) -> Result<()> {
		self.common.release_lock(cx)
	}

	#[ion(get)]
	pub fn get_closed(&self) -> *mut JSObject {
		self.common.closed.get()
	}
}