/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::iter;
use std::ptr;

use mozjs::gc::Traceable;
use mozjs::glue::JS_GetReservedSlot;
use mozjs::jsapi::{
	GCContext, GetRealmIteratorPrototype, Heap, JSClass, JSCLASS_BACKGROUND_FINALIZE, JSClassOps, JSContext,
	JSFunctionSpec, JSNativeWrapper, JSObject, JSTracer,
};
use mozjs::jsval::{JSVal, NullValue};

use crate::{Arguments, ClassDefinition, Context, Error, ErrorKind, Local, Object, ThrowException, Value};
use crate::class::{NativeClass, NativeObject, Reflector, TypeIdWrapper};
use crate::conversions::{IntoValue, ToValue};
use crate::flags::PropertyFlags;
use crate::functions::NativeFunction;
use crate::objects::class_reserved_slots;
use crate::spec::{create_function_spec, create_function_spec_symbol};
use crate::symbol::WellKnownSymbolCode;

pub trait JSIterator {
	fn next_value<'cx>(&mut self, cx: &'cx Context, private: &Value<'cx>) -> Option<Value<'cx>>;
}

impl<T, I: iter::Iterator<Item = T>> JSIterator for I
where
	T: for<'cx> IntoValue<'cx>,
{
	fn next_value<'cx>(&mut self, cx: &'cx Context, _: &Value) -> Option<Value<'cx>> {
		self.next().map(|val| {
			let mut rval = Value::undefined(cx);
			Box::new(val).into_value(cx, &mut rval);
			rval
		})
	}
}

pub struct IteratorResult<'cx> {
	value: Value<'cx>,
	done: bool,
}

impl<'cx> ToValue<'cx> for IteratorResult<'cx> {
	fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		let mut object = Object::new(cx);
		object.set_as(cx, "value", &self.value);
		object.set_as(cx, "done", &self.done);
		object.to_value(cx, value);
	}
}

pub struct Iterator {
	reflector: Reflector,
	iter: Box<dyn JSIterator>,
	private: Box<Heap<JSVal>>,
}

impl Iterator {
	pub fn new<I: JSIterator + 'static>(iter: I, private: &Value) -> Iterator {
		Iterator {
			reflector: Reflector::default(),
			iter: Box::new(iter),
			private: Heap::boxed(private.get()),
		}
	}

	pub fn next_value<'cx>(&mut self, cx: &'cx Context) -> IteratorResult<'cx> {
		let private = Value::from(unsafe { Local::from_heap(&self.private) });
		let next = self.iter.next_value(cx, &private);
		IteratorResult {
			done: next.is_none(),
			value: next.unwrap_or_else(|| Value::undefined(cx)),
		}
	}
}

impl Iterator {
	unsafe extern "C" fn constructor(cx: *mut JSContext, _: u32, _: *mut JSVal) -> bool {
		let cx = &unsafe { Context::new_unchecked(cx) };
		Error::new("Constructor should not be called", ErrorKind::Type).throw(cx);
		false
	}

	unsafe extern "C" fn next_raw(cx: *mut JSContext, argc: u32, vp: *mut JSVal) -> bool {
		let cx = &unsafe { Context::new_unchecked(cx) };
		let args = &mut unsafe { Arguments::new(cx, argc, vp) };

		let mut this = args.this().to_object(cx);
		let iterator = Iterator::get_mut_private(&mut this);
		let result = iterator.next_value(cx);

		result.to_value(cx, args.rval());
		true
	}

	unsafe extern "C" fn iterable(cx: *mut JSContext, argc: u32, vp: *mut JSVal) -> bool {
		let cx = &unsafe { Context::new_unchecked(cx) };
		let args = &mut unsafe { Arguments::new(cx, argc, vp) };

		let this = args.this().handle().get();
		args.rval().handle_mut().set(this);

		true
	}

	unsafe extern "C" fn finalise(_: *mut GCContext, this: *mut JSObject) {
		let mut value = NullValue();
		unsafe {
			JS_GetReservedSlot(this, 0, &mut value);
		}
		if value.is_double() && value.asBits_ & 0xFFFF000000000000 == 0 {
			let private = unsafe { &mut *(value.to_private() as *mut Option<Iterator>) };
			let _ = private.take();
		}
	}

	unsafe extern "C" fn trace(trc: *mut JSTracer, this: *mut JSObject) {
		let mut value = NullValue();
		unsafe {
			JS_GetReservedSlot(this, 0, &mut value);
		}
		if value.is_double() && value.asBits_ & 0xFFFF000000000000 == 0 {
			let private = unsafe { &*(value.to_private() as *mut Option<Iterator>) };
			unsafe {
				private.trace(trc);
			}
		}
	}
}

impl IntoValue<'_> for Iterator {
	fn into_value(self: Box<Self>, cx: &Context, value: &mut Value) {
		let object = cx.root_object(Iterator::new_object(cx, self));
		object.handle().get().to_value(cx, value);
	}
}

unsafe impl Traceable for Iterator {
	unsafe fn trace(&self, trc: *mut JSTracer) {
		unsafe {
			self.private.trace(trc);
		}
	}
}

static ITERATOR_CLASS_OPS: JSClassOps = JSClassOps {
	addProperty: None,
	delProperty: None,
	enumerate: None,
	newEnumerate: None,
	resolve: None,
	mayResolve: None,
	finalize: Some(Iterator::finalise),
	call: None,
	construct: None,
	trace: Some(Iterator::trace),
};

static ITERATOR_CLASS: NativeClass = NativeClass {
	base: JSClass {
		name: "NativeIterator\0".as_ptr().cast(),
		flags: JSCLASS_BACKGROUND_FINALIZE | class_reserved_slots(1),
		cOps: &ITERATOR_CLASS_OPS,
		spec: ptr::null_mut(),
		ext: ptr::null_mut(),
		oOps: ptr::null_mut(),
	},
	prototype_chain: [
		Some(&TypeIdWrapper::<Iterator>::new()),
		None,
		None,
		None,
		None,
		None,
		None,
		None,
	],
};

static ITERATOR_METHODS: &[JSFunctionSpec] = &[
	create_function_spec(
		"next\0",
		JSNativeWrapper {
			op: Some(Iterator::next_raw),
			info: ptr::null_mut(),
		},
		0,
		PropertyFlags::CONSTANT_ENUMERATED,
	),
	create_function_spec_symbol(
		WellKnownSymbolCode::Iterator,
		JSNativeWrapper {
			op: Some(Iterator::iterable),
			info: ptr::null_mut(),
		},
		0,
		PropertyFlags::CONSTANT,
	),
	JSFunctionSpec::ZERO,
];

impl NativeObject for Iterator {
	fn reflector(&self) -> &Reflector {
		&self.reflector
	}
}

impl ClassDefinition for Iterator {
	const NAME: &'static str = "";

	fn class() -> &'static NativeClass {
		&ITERATOR_CLASS
	}

	fn parent_class_info(cx: &Context) -> Option<(&'static NativeClass, Local<*mut JSObject>)> {
		Some((
			&ITERATOR_CLASS,
			cx.root_object(unsafe { GetRealmIteratorPrototype(cx.as_ptr()) }),
		))
	}

	fn constructor() -> (NativeFunction, u32) {
		(Iterator::constructor, 0)
	}

	fn functions() -> &'static [JSFunctionSpec] {
		ITERATOR_METHODS
	}
}
