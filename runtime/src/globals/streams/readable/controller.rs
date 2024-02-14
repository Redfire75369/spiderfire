/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::VecDeque;
use std::ptr;
use mozjs::conversions::ConversionBehavior;
use mozjs::gc::{GCMethods, HandleObject, RootKind};
use mozjs::jsapi::{Heap, Handle, JSContext, JSObject, JSFunction, Type};
use mozjs::jsval::{DoubleValue, Int32Value, JSVal, NullValue};
use ion::{
	Context, Exception, Function, Local, Object, Promise, Value, Result, ClassDefinition, Error, ErrorKind, ResultExc,
	TracedHeap,
};
use ion::class::{NativeObject, Reflector};
use ion::conversions::{FromValue, ToValue};
use ion::typedarray::{ArrayBuffer, ArrayBufferView, type_to_constructor, Uint8Array};
use crate::globals::streams::readable::reader::{Reader, ReaderKind, ByobReader, Request};
use crate::globals::streams::readable::{QueueingStrategy, State, UnderlyingSource};
use crate::globals::streams::readable::ReadableStream;

#[derive(Traceable)]
pub(crate) struct PullIntoDescriptor {
	pub(crate) buffer: Box<Heap<*mut JSObject>>,
	pub(crate) offset: usize,
	pub(crate) length: usize,
	pub(crate) filled: usize,
	pub(crate) element: usize,
	pub(crate) constructor: unsafe extern "C" fn(*mut JSContext, Handle<*mut JSObject>, usize, i64) -> *mut JSObject,
	pub(crate) kind: ReaderKind,
}

impl PullIntoDescriptor {
	pub(crate) fn buffer(&self) -> ArrayBuffer {
		ArrayBuffer::from(unsafe { Local::from_heap(&self.buffer) }).unwrap()
	}

	pub(crate) fn construct<'cx>(&self, cx: &'cx Context) -> ResultExc<ArrayBufferView<'cx>> {
		let view = unsafe {
			(self.constructor)(
				cx.as_ptr(),
				self.buffer.handle(),
				self.offset,
				(self.filled / self.element) as i64,
			)
		};

		if !view.is_null() {
			Ok(ArrayBufferView::from(cx.root(view)).unwrap())
		} else if let Some(exception) = Exception::new(cx)? {
			Err(exception)
		} else {
			Err(Error::new("Failed to Initialise Array Buffer", ErrorKind::Normal).into())
		}
	}

	pub(crate) fn commit(&mut self, cx: &Context, reader: &mut ByobReader, state: State) -> ResultExc<()> {
		let mut done = false;

		let buffer = self.buffer();
		if state == State::Closed {
			done = true;
		}
		let buffer = buffer.transfer(cx)?;

		self.buffer.set(buffer.get());
		let view = self.construct(cx)?.as_value(cx);

		let request = reader.common.requests.pop_front().unwrap();
		let promise = request.promise();

		if !done {
			(request.chunk)(cx, &promise, &view);
		} else {
			(request.close)(cx, &promise, Some(&view));
		}
		Ok(())
	}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Traceable)]
pub enum ControllerKind {
	Default,
	ByteStream,
}

pub enum Controller<'c> {
	Default(&'c mut DefaultController),
	ByteStream(&'c mut ByteStreamController),
}

impl Controller<'_> {
	pub(crate) fn common_mut(&mut self) -> &mut CommonController {
		match self {
			Controller::Default(controller) => &mut controller.common,
			Controller::ByteStream(controller) => &mut controller.common,
		}
	}

	pub fn cancel<'cx: 'v, 'v>(&mut self, cx: &'cx Context, reason: Option<Value<'v>>) -> ResultExc<Promise<'cx>> {
		let common = self.common_mut();
		let object = common.reflector.get();
		common.pull = None;
		common.queue_size = 0;
		let cancel = common.cancel.take();

		match self {
			Controller::Default(controller) => {
				controller.queue.clear();
				controller.size = None;
			}
			Controller::ByteStream(controller) => {
				controller.pending_descriptors.clear();
				controller.queue.clear();
			}
		}

		let mut promise = Promise::new(cx);
		if let Some(cancel) = &cancel {
			let cancel = Function::from(unsafe { Local::from_heap(cancel) });
			let this = Object::from(unsafe { Local::from_marked(&object) });
			let reason = reason.unwrap_or_else(Value::undefined_handle);
			let value = cancel.call(cx, &this, &[reason]).map_err(|report| report.unwrap().exception)?;
			if let Ok(result) = Promise::from_value(cx, &value, true, ()) {
				result.then(cx, |_, _| Ok(Value::undefined_handle()));
				promise.handle_mut().set(result.get());
			} else {
				promise.resolve(cx, &Value::undefined_handle());
			}
		}
		Ok(promise)
	}

	pub fn pull(&mut self, cx: &Context, promise: &Promise, request: Request) -> ResultExc<()> {
		match self {
			Controller::Default(controller) => {
				if let Some((chunk, _)) = controller.queue.pop_front() {
					if controller.common.close_requested && controller.queue.is_empty() {
						controller.common.pull = None;
						controller.common.cancel = None;
						controller.size = None;

						let stream = controller.common.stream(cx)?;
						stream.close(cx)?;
					} else {
						controller.pull_if_needed(cx)?;
					}

					let chunk = Value::from(unsafe { Local::from_heap(&chunk) });
					(request.chunk)(cx, promise, &chunk);
				} else {
					let stream = controller.common.stream(cx)?;
					match stream.native_reader(cx)? {
						Some(Reader::Default(reader)) => {
							if stream.state != State::Readable {
								return Err(Error::new("Cannot Add Read Request to Read Queue", None).into());
							} else {
								reader.common.requests.push_back(request);
							}
						}
						_ => return Ok(()),
					}
					controller.pull_if_needed(cx)?;
				}
			}
			Controller::ByteStream(controller) => {
				{
					let stream = controller.common.stream(cx)?;
					if stream.reader_kind != ReaderKind::Default {
						return Err(Error::new("Reader should have default reader.", ErrorKind::Type).into());
					}
				}

				if controller.common.queue_size > 0 {
					let (buffer, offset, length) = controller.queue.pop_front().unwrap();
					controller.common.queue_size -= length;

					if controller.common.queue_size == 0 && controller.common.close_requested {
						controller.close(cx)?;
					} else {
						controller.pull_if_needed(cx)?;
					}

					let buffer = ArrayBuffer::from(unsafe { Local::from_heap(&buffer) }).unwrap();
					let array = Uint8Array::with_array_buffer(cx, &buffer, offset, length).unwrap().as_value(cx);

					(request.chunk)(cx, promise, &array);
				} else {
					if controller.auto_allocate_chunk_size != 0 {
						let buffer = match ArrayBuffer::new(cx, controller.auto_allocate_chunk_size) {
							Some(buffer) => buffer,
							None => {
								controller.error_internal(cx, &Exception::new(cx).unwrap().as_value(cx))?;
								return Ok(());
							}
						};

						controller.pending_descriptors.push_back(PullIntoDescriptor {
							buffer: Heap::boxed(buffer.get()),
							offset: 0,
							length: controller.auto_allocate_chunk_size,
							filled: 0,
							element: 1,
							constructor: type_to_constructor(Type::Uint8),
							kind: ReaderKind::Default,
						});
					}

					let stream = controller.common.stream(cx)?;
					if let Some(Reader::Default(reader)) = stream.native_reader(cx)? {
						reader.common.requests.push_back(request);
					}
					controller.pull_if_needed(cx)?;
				}
			}
		}
		Ok(())
	}

	pub fn release(&mut self) {
		match self {
			Controller::Default(_) => {}
			Controller::ByteStream(controller) => {
				if let Some(descriptor) = controller.pending_descriptors.pop_front() {
					controller.pending_descriptors.clear();
					let buffer = descriptor.buffer.get();

					controller.pending_descriptors.push_back(descriptor);
					controller.pending_descriptors[0].buffer.set(buffer);
				}
			}
		}
	}
}

#[js_class]
pub struct CommonController {
	reflector: Reflector,

	pub(crate) stream: Box<Heap<*mut JSObject>>,
	pub(crate) underlying_source: Option<Box<Heap<*mut JSObject>>>,
	pub(crate) start: Option<Box<Heap<*mut JSFunction>>>,
	pub(crate) pull: Option<Box<Heap<*mut JSFunction>>>,
	pub(crate) cancel: Option<Box<Heap<*mut JSFunction>>>,

	pub(crate) started: bool,
	pub(crate) pulling: bool,
	pub(crate) pull_again: bool,
	pub(crate) close_requested: bool,

	high_water_mark: f64,
	pub(crate) queue_size: usize,
}

#[js_class]
impl CommonController {
	#[ion(constructor)]
	pub fn constructor() -> Result<CommonController> {
		unreachable!()
	}

	pub(crate) fn new(
		stream: &Object, source_object: Option<&Object>, source: &UnderlyingSource, high_water_mark: f64,
	) -> CommonController {
		fn to_heap<T>(obj: Option<&Local<T>>) -> Option<Box<Heap<T>>>
		where
			T: GCMethods + RootKind + Copy,
			Heap<T>: Default,
		{
			obj.as_ref().map(|s| Heap::boxed(s.get()))
		}

		CommonController {
			reflector: Reflector::default(),

			stream: Heap::boxed(stream.handle().get()),
			underlying_source: to_heap(source_object.map(|o| &**o)),
			start: to_heap(source.start.as_deref()),
			pull: to_heap(source.pull.as_deref()),
			cancel: to_heap(source.cancel.as_deref()),

			started: false,
			pulling: false,
			pull_again: false,
			close_requested: false,

			high_water_mark,
			queue_size: 0,
		}
	}

	pub(crate) fn stream<'cx>(&self, cx: &'cx Context) -> Result<&'cx mut ReadableStream> {
		let stream = Object::from(unsafe { Local::from_heap(&*ptr::from_ref(&self.stream)) });
		ReadableStream::get_mut_private(cx, &stream)
	}

	pub(crate) fn underlying_source(&self) -> Object {
		self.underlying_source
			.as_ref()
			.map(|s| Object::from(unsafe { Local::from_heap(s) }))
			.unwrap_or_else(|| Object::from(Local::from_handle(HandleObject::null())))
	}

	pub(crate) fn pull(&self) -> Option<Function> {
		self.pull.as_ref().map(|pull| Function::from(unsafe { Local::from_heap(pull) }))
	}

	pub(crate) fn start<C: ControllerInternals>(&mut self, cx: &Context, start: Option<&Function>) {
		let controller = self.reflector().get();

		if let Some(start) = start {
			let underlying_source = self.underlying_source();
			let value = controller.as_value(cx);
			let result = start.call(cx, &underlying_source, &[value]).map(|v| v.get());

			let promise = match result {
				Ok(value) => Promise::resolved(cx, &Value::from(cx.root(value))),
				Err(Some(report)) => Promise::rejected(cx, &report.exception.as_value(cx)),
				Err(None) => unreachable!(),
			};

			let controller1 = TracedHeap::new(controller);
			let controller2 = TracedHeap::new(controller);
			promise.add_reactions(
				cx,
				move |cx, _| {
					let controller = C::from_traced_heap(cx, &controller1)?;
					controller.common().started = true;
					controller.pull_if_needed(cx)?;
					Ok(Value::undefined_handle())
				},
				move |cx, error| {
					let controller = C::from_traced_heap(cx, &controller2)?;
					controller.error_internal(cx, error)?;
					Ok(Value::undefined_handle())
				},
			);
		}
	}

	pub(crate) fn can_close_or_enqueue(&self, stream: &ReadableStream) -> bool {
		stream.state == State::Readable && !self.close_requested
	}

	pub(crate) fn should_call_pull(&self, cx: &Context, stream: &ReadableStream) -> Result<bool> {
		if !self.can_close_or_enqueue(stream) || !self.started {
			return Ok(false);
		}

		if let Some(reader) = stream.native_reader(cx)? {
			if !reader.is_empty() {
				return Ok(true);
			}
		}
		Ok(stream.state == State::Readable && self.high_water_mark > self.queue_size as f64)
	}

	pub(crate) fn pull_if_needed<C: ControllerInternals>(&mut self, cx: &Context) -> ResultExc<()> {
		let stream = self.stream(cx)?;
		if !self.should_call_pull(cx, stream)? {
			return Ok(());
		}

		if self.pulling {
			self.pull_again = true;
			return Ok(());
		}

		self.pulling = true;

		if let Some(pull) = &self.pull() {
			let controller = stream.controller.get().as_value(cx);
			let this = self.underlying_source();
			let result = pull.call(cx, &this, &[controller]).map_err(|report| report.unwrap().exception)?;

			let promise = match Promise::from_value(cx, &result, true, ()) {
				Ok(promise) => promise,
				Err(_) => Promise::new(cx),
			};

			let controller1 = TracedHeap::new(stream.controller.get());
			let controller2 = TracedHeap::new(stream.controller.get());
			promise.add_reactions(
				cx,
				move |cx, _| {
					let controller = C::from_traced_heap(cx, &controller1)?;
					controller.common().pulling = false;
					if controller.common().pull_again {
						controller.common().pull_again = false;
						controller.pull_if_needed(cx)?;
					}
					Ok(Value::undefined_handle())
				},
				move |cx, error| {
					let controller = C::from_traced_heap(cx, &controller2)?;
					controller.error_internal(cx, error)?;
					Ok(Value::undefined_handle())
				},
			);
		}
		Ok(())
	}

	pub(crate) fn desired_size(&self, cx: &Context) -> Result<JSVal> {
		let size = match self.stream(cx)?.state {
			State::Readable => DoubleValue(self.high_water_mark - self.queue_size as f64),
			State::Closed => Int32Value(0),
			State::Errored => NullValue(),
		};
		Ok(size)
	}
}

pub(crate) trait ControllerInternals: ClassDefinition {
	fn from_traced_heap<'h>(cx: &Context, heap: &'h TracedHeap<*mut JSObject>) -> Result<&'h mut Self> {
		let controller = Object::from(heap.to_local());
		Self::get_mut_private(cx, &controller)
	}

	fn common(&mut self) -> &mut CommonController;

	fn start(&mut self, cx: &Context, start: Option<&Function>) {
		self.common().start::<Self>(cx, start);
	}

	fn pull_if_needed(&mut self, cx: &Context) -> ResultExc<()> {
		self.common().pull_if_needed::<Self>(cx)
	}

	fn clear(&mut self);

	fn error_internal(&mut self, cx: &Context, error: &Value) -> Result<()> {
		if self.common().stream(cx)?.state == State::Readable {
			self.clear();
			self.common().stream(cx)?.error(cx, error)
		} else {
			Ok(())
		}
	}
}

fn controller_error<C: ControllerInternals>(cx: &Context, controller: &mut C, error: Option<Value>) -> Result<()> {
	controller.error_internal(cx, &error.unwrap_or_else(Value::undefined_handle))
}

#[js_class]
#[ion(name = "ReadableStreamDefaultController")]
pub struct DefaultController {
	pub(crate) common: CommonController,
	pub(crate) size: Option<Box<Heap<*mut JSFunction>>>,
	pub(crate) queue: VecDeque<(Box<Heap<JSVal>>, u64)>,
}

#[js_class]
impl DefaultController {
	#[ion(constructor)]
	pub fn constructor() -> Result<DefaultController> {
		Err(Error::new(
			"ReadableStreamDefaultController has no constructor.",
			ErrorKind::Type,
		))
	}

	pub(crate) fn initialise(
		stream: &Object, source_object: Option<&Object>, source: &UnderlyingSource, strategy: &QueueingStrategy,
		high_water_mark: f64,
	) -> DefaultController {
		let size = strategy.size.as_ref().map(|s| Heap::boxed(s.get()));

		DefaultController {
			common: CommonController::new(stream, source_object, source, high_water_mark),
			size,
			queue: VecDeque::new(),
		}
	}

	#[ion(get)]
	pub fn get_desired_size(&self, cx: &Context) -> Result<JSVal> {
		self.common.desired_size(cx)
	}

	pub fn close(&mut self, cx: &Context) -> Result<()> {
		let stream = self.common.stream(cx)?;
		if self.common.can_close_or_enqueue(stream) {
			if self.queue.is_empty() {
				self.common.close_requested = true;
			}
			self.common.pull = None;
			self.common.cancel = None;
			self.size = None;

			stream.close(cx)
		} else {
			Err(Error::new("Cannot Close Stream", ErrorKind::Type))
		}
	}

	pub fn enqueue(&mut self, cx: &Context, chunk: Value) -> ResultExc<()> {
		let stream = self.common.stream(cx)?;
		if self.common.can_close_or_enqueue(stream) {
			if let Some(Reader::Default(reader)) = stream.native_reader(cx)? {
				if let Some(request) = reader.common.requests.pop_front() {
					let promise = request.promise();
					(request.chunk)(cx, &promise, &chunk);
					return Ok(());
				}
			}
			let args = &[chunk];
			let result = self
				.size
				.as_ref()
				.map(|size| {
					let size = Function::from(unsafe { Local::from_heap(size) });
					size.call(cx, &Object::null(cx), args)
				})
				.unwrap_or_else(|| Ok(Value::i32(cx, 1)));
			match result {
				Ok(size) => {
					let size = u64::from_value(cx, &size, false, ConversionBehavior::EnforceRange);
					match size {
						Ok(size) => {
							self.queue.push_back((Heap::boxed(args[0].get()), size));
							self.common.queue_size += size as usize;
							self.pull_if_needed(cx)?;
						}
						Err(error) => {
							self.error_internal(cx, &error.as_value(cx))?;
						}
					}
				}
				Err(Some(report)) => {
					self.error_internal(cx, &report.exception.as_value(cx))?;
				}
				Err(None) => unreachable!(),
			}
			Ok(())
		} else {
			Err(Error::new("Cannot Enqueue to Stream", ErrorKind::Type).into())
		}
	}

	pub fn error(&mut self, cx: &Context, error: Option<Value>) -> Result<()> {
		controller_error(cx, self, error)
	}
}

impl ControllerInternals for DefaultController {
	fn common(&mut self) -> &mut CommonController {
		&mut self.common
	}

	fn clear(&mut self) {
		self.queue.clear();
		self.common.queue_size = 0;
		self.common.pull = None;
		self.common.cancel = None;
		self.size = None;
	}
}

#[js_class]
#[ion(name = "ReadableByteStreamController")]
pub struct ByteStreamController {
	pub(crate) common: CommonController,
	pub(crate) auto_allocate_chunk_size: usize,
	pub(crate) pending_descriptors: VecDeque<PullIntoDescriptor>,
	pub(crate) queue: VecDeque<(Box<Heap<*mut JSObject>>, usize, usize)>,
}

#[js_class]
impl ByteStreamController {
	#[ion(constructor)]
	pub fn constructor() -> Result<ByteStreamController> {
		Err(Error::new(
			"ReadableByteStreamController has no constructor.",
			ErrorKind::Type,
		))
	}

	pub(crate) fn initialise(
		stream: &Object, source_object: &Object, source: &UnderlyingSource, high_water_mark: f64,
	) -> Result<ByteStreamController> {
		if let Some(auto_allocate_chunk_size) = source.auto_allocate_chunk_size {
			if auto_allocate_chunk_size == 0 {
				return Err(Error::new("autoAllocateChunkSize can not be zero.", ErrorKind::Type));
			}
		}

		Ok(ByteStreamController {
			common: CommonController::new(stream, Some(source_object), source, high_water_mark),
			auto_allocate_chunk_size: source.auto_allocate_chunk_size.unwrap_or(0) as usize,
			pending_descriptors: VecDeque::new(),
			queue: VecDeque::new(),
		})
	}

	pub(crate) fn fill_pull_into_descriptor(
		&mut self, cx: &Context, descriptor: &mut PullIntoDescriptor,
	) -> ResultExc<bool> {
		let aligned = descriptor.filled - descriptor.filled % descriptor.element;
		let max_copy = self.common.queue_size.min(descriptor.length - descriptor.filled);
		let max_aligned = descriptor.filled + max_copy - (descriptor.filled + max_copy) % descriptor.element;

		let ready = max_aligned > aligned;

		let mut remaining = if ready {
			max_aligned - descriptor.filled
		} else {
			max_copy
		};

		while remaining > 0 {
			let mut copy = remaining;
			let mut len = 0;

			if let Some((chunk, offset, length)) = self.queue.front() {
				copy = copy.min(*length);
				len = *length;
				let chunk = ArrayBuffer::from(unsafe { Local::from_heap(chunk) }).unwrap();
				let buffer = descriptor.buffer();
				if !buffer.copy_data_to(cx, &chunk, copy, descriptor.offset + descriptor.filled, *offset) {
					return Err(Exception::new(cx)?.unwrap());
				}
			}

			if copy == len {
				self.queue.pop_front();
			} else if let Some((_, offset, length)) = self.queue.get_mut(0) {
				*offset += copy;
				*length -= copy;
			}
			self.common.queue_size -= copy;
			descriptor.filled += copy;
			remaining -= copy;
		}

		if !ready {
			// TODO: Assert Queue Size 0, Assert Filled > 0, Assert Filled < Element Size
		}

		Ok(ready)
	}

	#[ion(get)]
	pub fn get_desired_size(&self, cx: &Context) -> Result<JSVal> {
		self.common.desired_size(cx)
	}

	pub fn close(&mut self, cx: &Context) -> Result<()> {
		let stream = self.common.stream(cx)?;
		if self.common.can_close_or_enqueue(stream) {
			if self.common.queue_size > 0 {
				self.common.close_requested = true;
			}
			if let Some(descriptor) = self.pending_descriptors.front() {
				if descriptor.filled % descriptor.element > 0 {
					let error = Error::new("Pending Pull-Into Not Empty", ErrorKind::Type);
					self.error_internal(cx, &error.as_value(cx))?;
					return Err(error);
				}
			}

			self.common.pull = None;
			self.common.cancel = None;
			stream.close(cx)
		} else {
			Err(Error::new("Cannot Close Byte Stream Controller", ErrorKind::Type))
		}
	}

	pub fn enqueue(&mut self, cx: &Context, chunk: ArrayBufferView) -> ResultExc<()> {
		if chunk.data().1 == 0 {
			return Err(Error::new("Chunk must contain bytes.", ErrorKind::Type).into());
		}

		let buffer = chunk.buffer(cx);

		if buffer.data().1 == 0 {
			return Err(Error::new("Chunk must contain bytes.", ErrorKind::Type).into());
		}

		let stream = self.common.stream(cx)?;
		if self.common.can_close_or_enqueue(stream) {
			let buffer = buffer.transfer(cx)?;
			let offset = chunk.offset();

			let mut shift = false;
			if let Some(descriptor) = self.pending_descriptors.get_mut(0) {
				let buffer = descriptor.buffer().transfer(cx)?;
				descriptor.buffer.set(buffer.get());
				if descriptor.kind == ReaderKind::None && descriptor.filled > 0 {
					let buffer = match buffer.clone(cx, descriptor.offset, descriptor.length) {
						Some(buffer) => buffer,
						None => {
							let exception = Exception::new(cx)?.unwrap();
							self.error_internal(cx, &exception.as_value(cx))?;
							return Err(exception);
						}
					};

					self.queue.push_back((Heap::boxed(buffer.get()), 0, descriptor.length));
					self.common.queue_size += descriptor.length;
					shift = true;
				}
			}

			if shift {
				self.pending_descriptors.pop_front();
			}

			match stream.native_reader(cx)? {
				Some(Reader::Default(reader)) => {
					let mut complete = false;
					while let Some(request) = reader.common.requests.pop_front() {
						let promise = request.promise();

						if self.common.queue_size == 0 {
							self.pending_descriptors.pop_front();

							let array = Uint8Array::with_array_buffer(cx, &buffer, offset, chunk.data().1)
								.unwrap()
								.as_value(cx);
							(request.chunk)(cx, &promise, &array);

							complete = true;
							break;
						}

						let (buffer, offset, length) = self.queue.pop_front().unwrap();
						self.common.queue_size -= length;

						if self.common.queue_size == 0 && self.common.close_requested {
							self.close(cx)?;
						} else {
							self.pull_if_needed(cx)?;
						}

						let buffer = ArrayBuffer::from(unsafe { Local::from_heap(&buffer) }).unwrap();
						let array = Uint8Array::with_array_buffer(cx, &buffer, offset, length).unwrap().as_value(cx);

						(request.chunk)(cx, &promise, &array);
					}

					if !complete {
						self.queue.push_back((Heap::boxed(buffer.get()), offset, chunk.data().1));
						self.common.queue_size += chunk.data().1;
					}
				}
				Some(Reader::Byob(reader)) => {
					self.queue.push_back((Heap::boxed(buffer.get()), offset, chunk.data().1));
					self.common.queue_size += chunk.data().1;

					while !self.pending_descriptors.is_empty() {
						if self.common.queue_size == 0 {
							break;
						}

						let mut shift = false;

						let descriptor = ptr::from_mut(self.pending_descriptors.get_mut(0).unwrap());
						if self.fill_pull_into_descriptor(cx, unsafe { &mut *descriptor })? {
							shift = true;
						}

						if shift {
							let mut descriptor = self.pending_descriptors.pop_front().unwrap();
							descriptor.commit(cx, reader, stream.state)?;
						}
					}
				}
				None => {
					self.queue.push_back((Heap::boxed(buffer.get()), offset, chunk.data().1));
					self.common.queue_size += chunk.data().1;
				}
			}
			self.pull_if_needed(cx)
		} else {
			Err(Error::new("Cannot Enqueue to Stream", ErrorKind::Type).into())
		}
	}

	pub fn error(&mut self, cx: &Context, error: Option<Value>) -> Result<()> {
		controller_error(cx, self, error)
	}
}

impl ControllerInternals for ByteStreamController {
	fn common(&mut self) -> &mut CommonController {
		&mut self.common
	}

	fn clear(&mut self) {
		self.pending_descriptors.clear();
		self.queue.clear();
		self.common.queue_size = 0;
		self.common.pull = None;
		self.common.cancel = None;
	}
}
