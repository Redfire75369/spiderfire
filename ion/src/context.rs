/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::ptr::NonNull;

use mozjs::gc::{GCMethods, RootedTraceableSet};
use mozjs::jsapi::{
	BigInt, Heap, JS_GetContextPrivate, JS_SetContextPrivate, JSContext, JSFunction, JSObject, JSScript, JSString, PropertyDescriptor, PropertyKey,
	Rooted, Symbol,
};
use mozjs::jsval::JSVal;
use mozjs::rust::Runtime;
use typed_arena::Arena;

use crate::class::ClassInfo;
use crate::Local;
use crate::module::ModuleLoader;

/// Represents Types that can be Rooted in SpiderMonkey
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
pub struct Context {
	context: NonNull<JSContext>,
	rooted: RootedArena,
	order: RefCell<Vec<GCType>>,
	private: NonNull<ContextInner>,
}

impl Context {
	pub fn from_runtime(rt: &Runtime) -> Context {
		let cx = rt.cx();

		let private = NonNull::new(unsafe { JS_GetContextPrivate(cx).cast() }).unwrap_or_else(|| {
			let private = Box::<ContextInner>::default();
			let private = Box::into_raw(private);
			unsafe {
				JS_SetContextPrivate(cx, private.cast());
			}
			unsafe { NonNull::new_unchecked(private) }
		});

		Context {
			context: unsafe { NonNull::new_unchecked(cx) },
			rooted: RootedArena::default(),
			order: RefCell::new(Vec::new()),
			private,
		}
	}

	pub unsafe fn new_unchecked(cx: *mut JSContext) -> Context {
		Context {
			context: unsafe { NonNull::new_unchecked(cx) },
			rooted: RootedArena::default(),
			order: RefCell::new(Vec::new()),
			private: unsafe { NonNull::new_unchecked(JS_GetContextPrivate(cx).cast()) },
		}
	}

	pub fn as_ptr(&self) -> *mut JSContext {
		self.context.as_ptr()
	}

	pub fn get_inner_data(&self) -> NonNull<ContextInner> {
		self.private
	}

	pub fn get_raw_private(&self) -> *mut c_void {
		let inner = self.get_inner_data();
		unsafe { (*inner.as_ptr()).private }
	}

	pub fn set_private<T>(&self, private: Box<T>) {
		let inner_private = self.get_inner_data();
		unsafe {
			(*inner_private.as_ptr()).private = Box::into_raw(private).cast();
		}
	}
}

macro_rules! impl_root_methods {
	($(($fn_name:ident, $pointer:ty, $key:ident, $gc_type:ident)$(,)?)*) => {
		$(
			#[doc = concat!("Roots a [", stringify!($pointer), "](", stringify!($pointer), ") as a ", stringify!($gc_type), " ands returns a [Local] to it.")]
			pub fn $fn_name(&self, ptr: $pointer) -> Local<$pointer> {
				let root = self.rooted.$key.alloc(Rooted::new_unrooted());
				self.order.borrow_mut().push(GCType::$gc_type);

				Local::new(self, root, ptr)
			}
		)*
	};
	([persistent], $(($root_fn:ident, $unroot_fn:ident, $pointer:ty, $key:ident)$(,)?)*) => {
		$(
			pub fn $root_fn(&self, ptr: $pointer) -> Local<$pointer> {
				let heap = Heap::boxed(ptr);
				let persistent = unsafe { &mut (*self.get_inner_data().as_ptr()).persistent.$key };
				persistent.push(heap);
				let ptr = &*persistent[persistent.len() - 1];
				unsafe {
					RootedTraceableSet::add(ptr);
					Local::from_heap(ptr)
				}
			}

			pub fn $unroot_fn(&self, ptr: $pointer) {
				let persistent = unsafe { &mut (*self.get_inner_data().as_ptr()).persistent.$key };
				let idx = match persistent.iter().rposition(|x| x.get() == ptr) {
					Some(idx) => idx,
					None => return,
				};
				unsafe {
					RootedTraceableSet::remove(&*persistent[idx]);
				}
				persistent.swap_remove(idx);
			}
		)*
	};
}

impl Context {
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
		[persistent],
		(root_persistent_object, unroot_persistent_object, *mut JSObject, objects)
	}
}

macro_rules! impl_drop {
	([$self:expr], $(($key:ident, $gc_type:ident)$(,)?)*) => {
		$(let $key: Vec<_> = $self.rooted.$key.iter_mut().collect();)*
		$(let mut $key = $key.into_iter().rev();)*

		for ty in $self.order.take().into_iter().rev() {
			match ty {
				$(
					GCType::$gc_type => {
						let root = $key.next().unwrap();
						root.ptr = unsafe { GCMethods::initial() };
						unsafe {
							root.remove_from_root_stack();
						}
					}
				)*
			}
		}
	}
}

impl Drop for Context {
	/// Drops the rooted values in reverse-order to maintain LIFO destruction in the Linked List.
	fn drop(&mut self) {
		impl_drop! {
			[self],
			(values, Value),
			(objects, Object),
			(strings, String),
			(scripts, Script),
			(property_keys, PropertyKey),
			(property_descriptors, PropertyDescriptor),
			(functions, Function),
			(big_ints, BigInt),
			(symbols, Symbol),
		}
	}
}
