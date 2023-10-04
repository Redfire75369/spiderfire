/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::{take, transmute};
use std::ptr;
use std::ptr::NonNull;

use mozjs::gc::RootedTraceableSet;
use mozjs::jsapi::{
	BigInt, Heap, JS_GetContextPrivate, JS_SetContextPrivate, JSContext, JSFunction, JSObject, JSScript, JSString, PropertyDescriptor, PropertyKey,
	Rooted, Symbol,
};
use mozjs::jsval::JSVal;
use mozjs::rust::{RootedGuard, Runtime};
use typed_arena::Arena;

use crate::class::ClassInfo;
use crate::Local;
use crate::module::ModuleLoader;

/// Represents Types that can be Rooted in SpiderMonkey
#[allow(dead_code)]
pub enum GCType {
	Value,
	Object,
	String,
	Script,
	PropertyKey,
	PropertyDescriptor,
	Function,
	BigInt,
	Symbol,
}

/// Holds Rooted Values
#[derive(Default)]
struct RootedArena {
	values: Arena<Rooted<JSVal>>,
	objects: Arena<Rooted<*mut JSObject>>,
	strings: Arena<Rooted<*mut JSString>>,
	scripts: Arena<Rooted<*mut JSScript>>,
	property_keys: Arena<Rooted<PropertyKey>>,
	property_descriptors: Arena<Rooted<PropertyDescriptor>>,
	functions: Arena<Rooted<*mut JSFunction>>,
	big_ints: Arena<Rooted<*mut BigInt>>,
	symbols: Arena<Rooted<*mut Symbol>>,
}

/// Holds RootedGuards which have been converted to [Local]s
#[derive(Default)]
struct LocalArena<'a> {
	order: RefCell<Vec<GCType>>,
	values: Arena<RootedGuard<'a, JSVal>>,
	objects: Arena<RootedGuard<'a, *mut JSObject>>,
	strings: Arena<RootedGuard<'a, *mut JSString>>,
	scripts: Arena<RootedGuard<'a, *mut JSScript>>,
	property_keys: Arena<RootedGuard<'a, PropertyKey>>,
	property_descriptors: Arena<RootedGuard<'a, PropertyDescriptor>>,
	functions: Arena<RootedGuard<'a, *mut JSFunction>>,
	big_ints: Arena<RootedGuard<'a, *mut BigInt>>,
	symbols: Arena<RootedGuard<'a, *mut Symbol>>,
}

thread_local!(static HEAP_OBJECTS: RefCell<Vec<Heap<*mut JSObject>>> = RefCell::new(Vec::new()));

#[allow(clippy::vec_box)]
#[derive(Default)]
pub struct Persistent {
	objects: Vec<Box<Heap<*mut JSObject>>>,
}

pub struct ContextInner {
	pub class_infos: HashMap<TypeId, ClassInfo>,
	pub module_loader: Option<Box<dyn ModuleLoader>>,
	persistent: Persistent,
	private: *mut c_void,
}

impl Default for ContextInner {
	fn default() -> ContextInner {
		ContextInner {
			class_infos: HashMap::new(),
			module_loader: None,
			persistent: Persistent::default(),
			private: ptr::null_mut(),
		}
	}
}

/// Represents the thread-local state of the runtime.
///
/// Wrapper around [JSContext] that provides lifetime information and convenient APIs.
pub struct Context<'c> {
	context: NonNull<JSContext>,
	rooted: RootedArena,
	local: LocalArena<'static>,
	private: OnceCell<*mut ContextInner>,
	_lifetime: PhantomData<&'c ()>,
}

impl<'c> Context<'c> {
	pub fn from_runtime(rt: &Runtime) -> Context<'c> {
		let cx = rt.cx();

		if unsafe { JS_GetContextPrivate(cx).is_null() } {
			let inner_private = Box::<ContextInner>::default();
			let inner_private = Box::into_raw(inner_private);
			unsafe {
				JS_SetContextPrivate(cx, inner_private.cast());
			}
		}

		Context {
			context: unsafe { NonNull::new_unchecked(cx) },
			rooted: RootedArena::default(),
			local: LocalArena::default(),
			private: OnceCell::new(),
			_lifetime: PhantomData,
		}
	}

	pub unsafe fn new_unchecked(context: *mut JSContext) -> Context<'c> {
		Context {
			context: unsafe { NonNull::new_unchecked(context) },
			rooted: RootedArena::default(),
			local: LocalArena::default(),
			private: OnceCell::new(),
			_lifetime: PhantomData,
		}
	}

	pub fn as_ptr(&self) -> *mut JSContext {
		self.context.as_ptr()
	}

	pub fn get_inner_data(&self) -> *mut ContextInner {
		*self.private.get_or_init(|| unsafe { JS_GetContextPrivate(self.as_ptr()).cast() })
	}

	pub fn get_raw_private(&self) -> *mut c_void {
		let inner = self.get_inner_data();
		if inner.is_null() {
			ptr::null_mut()
		} else {
			unsafe { (*inner).private }
		}
	}

	pub fn set_raw_private(&self, private: *mut c_void) {
		let inner_private = self.get_inner_data();
		if !inner_private.is_null() {
			unsafe {
				(*inner_private).private = private;
			}
		}
	}
}

macro_rules! impl_root_methods {
	($(($fn_name:ident, $pointer:ty, $key:ident, $gc_type:ident)$(,)?)*) => {
		$(
			#[doc = concat!("Roots a [", stringify!($pointer), "](", stringify!($pointer), ") as a ", stringify!($gc_type), " ands returns a [Local] to it.")]
			pub fn $fn_name(&self, ptr: $pointer) -> Local<$pointer> {
				let rooted = self.rooted.$key.alloc(Rooted::new_unrooted());
				self.local.order.borrow_mut().push(GCType::$gc_type);

				Local::from_rooted(
					unsafe {
						transmute(self.local.$key.alloc(RootedGuard::new(self.as_ptr(), transmute(rooted), ptr)))
					}
				)
			}
		)*
	};
	(persistent $(($root_fn:ident, $unroot_fn:ident, $pointer:ty, $key:ident)$(,)?)*) => {
		$(
			pub fn $root_fn(&self, ptr: $pointer) -> Local<$pointer> {
				let heap = Heap::boxed(ptr);
				let persistent = unsafe { &mut (*self.get_inner_data()).persistent.$key };
				persistent.push(heap);
				let ptr = &persistent[persistent.len() - 1];
				unsafe {
					RootedTraceableSet::add(ptr);
					Local::from_heap(ptr)
				}
			}

			pub fn $unroot_fn(&self, ptr: $pointer) {
				let persistent = unsafe { &mut (*self.get_inner_data()).persistent.$key };
				let idx = match persistent.iter().rposition(|x| x.get() == ptr) {
					Some(idx) => idx,
					None => return,
				};
				let heap = persistent.swap_remove(idx);
				unsafe {
					RootedTraceableSet::remove(&heap);
				}
			}
		)*
	};
}

impl Context<'_> {
	impl_root_methods! {
		(root_value, JSVal, values, Value),
		(root_object, *mut JSObject, objects, Object),
		(root_string, *mut JSString, strings, String),
		(root_script, *mut JSScript, scripts, Script),
		(root_property_key, PropertyKey, property_keys, PropertyKey),
		(root_property_descriptor, PropertyDescriptor, property_descriptors, PropertyDescriptor),
		(root_function, *mut JSFunction, functions, Function),
		(root_bigint, *mut BigInt, big_ints, BigInt),
		(root_symbol, *mut Symbol, symbols, Symbol),
	}

	impl_root_methods! {
		persistent
		(root_persistent_object, unroot_persistent_object, *mut JSObject, objects)
	}
}

macro_rules! impl_drop {
	([$self:expr], $(($pointer:ty, $key:ident, $gc_type:ident)$(,)?)*) => {
		$(let $key = take(&mut $self.local.$key);)*

		$(let mut $key = $key.into_vec().into_iter().rev();)*

		for ty in $self.local.order.take().into_iter().rev() {
			match ty {
				$(
					GCType::$gc_type => {
						let _ = $key.next();
					}
				)*
			}
		}
	}
}

impl Drop for Context<'_> {
	/// Drops the rooted values in reverse-order to maintain LIFO destruction in the Linked List.
	fn drop(&mut self) {
		impl_drop! {
			[self],
			(JSVal, values, Value),
			(*mut JSObject, objects, Object),
			(*mut JSString, strings, String),
			(*mut JSScript, scripts, Script),
			(PropertyKey, property_keys, PropertyKey),
			(PropertyDescriptor, property_descriptors, PropertyDescriptor),
			(*mut JSFunction, functions, Function),
			(*mut BigInt, big_ints, BigInt),
			(*mut Symbol, symbols, Symbol),
		}
	}
}
