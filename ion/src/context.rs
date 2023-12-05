/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::ptr::NonNull;

use mozjs::gc::{RootedTraceableSet, Traceable};
use mozjs::gc::GCMethods;
use mozjs::jsapi::{
	BigInt, Heap, JS_GetContextPrivate, JS_SetContextPrivate, JSContext, JSFunction, JSObject, JSScript, JSString,
	PropertyKey, Symbol,
};
use mozjs::jsval::JSVal;
use mozjs::rust::Runtime;

use crate::class::ClassInfo;
use crate::module::ModuleLoader;
use crate::Root;

pub trait StableTraceable
where
	Heap<Self::Traced>: Traceable,
{
	type Traced: Copy + GCMethods + 'static;

	fn heap(&self) -> &Heap<Self::Traced>;
}

impl<T: Copy + GCMethods + 'static> StableTraceable for Box<Heap<T>>
where
	Heap<T>: Traceable,
{
	type Traced = T;

	fn heap(&self) -> &Heap<T> {
		self
	}
}

#[allow(clippy::vec_box)]
#[derive(Default)]
pub struct RootCollection(UnsafeCell<Vec<*const dyn Traceable>>);

impl RootCollection {
	pub(crate) unsafe fn root(&self, traceable: *const dyn Traceable) {
		unsafe {
			RootedTraceableSet::add(traceable);
			(*self.0.get()).push(traceable);
		}
	}

	pub(crate) unsafe fn unroot(&self, traceable: *const dyn Traceable) -> bool {
		unsafe {
			let collection = &mut *self.0.get();
			match collection.iter().rposition(|r| ptr::eq(*r as *const u8, traceable as *const u8)) {
				Some(index) => {
					RootedTraceableSet::remove(traceable);
					collection.swap_remove(index);
					true
				}
				None => false,
			}
		}
	}
}

pub struct ContextInner {
	pub class_infos: HashMap<TypeId, ClassInfo>,
	pub module_loader: Option<Box<dyn ModuleLoader>>,
	roots: RootCollection,
	private: *mut c_void,
}

impl Default for ContextInner {
	fn default() -> ContextInner {
		ContextInner {
			class_infos: HashMap::new(),
			module_loader: None,
			roots: RootCollection::default(),
			private: ptr::null_mut(),
		}
	}
}

/// Represents the thread-local state of the runtime.
///
/// Wrapper around [JSContext] that provides lifetime information and convenient APIs.
pub struct Context {
	context: NonNull<JSContext>,
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
			private,
		}
	}

	pub unsafe fn new_unchecked(cx: *mut JSContext) -> Context {
		Context {
			context: unsafe { NonNull::new_unchecked(cx) },
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
			#[deprecated]
			pub fn $fn_name(&self, ptr: $pointer) -> Root<Box<Heap<$pointer>>> {
				self.root(ptr)
			}
		)*
	};
}

impl Context {
	pub fn root<T: Copy + GCMethods + 'static>(&self, value: T) -> Root<Box<Heap<T>>>
	where
		Heap<T>: Traceable,
	{
		let heap = Box::new(Heap {
			ptr: UnsafeCell::new(unsafe { T::initial() }),
		});
		heap.set(value);

		unsafe {
			let roots = &(*self.get_inner_data().as_ptr()).roots;
			roots.root(heap.heap());
			Root::new(heap, roots)
		}
	}

	// TODO: (root_property_descriptor, PropertyDescriptor, property_descriptors, PropertyDescriptor),
	impl_root_methods! {
		(root_value, JSVal, values, Value),
		(root_object, *mut JSObject, objects, Object),
		(root_string, *mut JSString, strings, String),
		(root_script, *mut JSScript, scripts, Script),
		(root_property_key, PropertyKey, property_keys, PropertyKey),
		(root_function, *mut JSFunction, functions, Function),
		(root_bigint, *mut BigInt, big_ints, BigInt),
		(root_symbol, *mut Symbol, symbols, Symbol),
	}
}
