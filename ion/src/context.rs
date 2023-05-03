/*c
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::mem::{take, transmute};
use std::ops::Deref;
use std::ptr;

use mozjs::gc::RootedTraceableSet;
use mozjs::jsapi::{Heap, JSContext, JSFunction, JSObject, JSScript, JSString, PropertyKey, Rooted, Symbol};
use mozjs::jsval::JSVal;
use mozjs::rust::RootedGuard;
use typed_arena::Arena;

use crate::Local;

/// Represents Types that can be Rooted in SpiderMonkey
#[allow(dead_code)]
pub enum GCType {
	Value,
	Object,
	String,
	Script,
	PropertyKey,
	Function,
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
	functions: Arena<Rooted<*mut JSFunction>>,
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
	functions: Arena<RootedGuard<'a, *mut JSFunction>>,
	symbols: Arena<RootedGuard<'a, *mut Symbol>>,
}

thread_local!(static HEAP_OBJECTS: RefCell<Vec<Heap<*mut JSObject>>> = RefCell::new(Vec::new()));

/// Represents the thread-local state of the runtime.
///
/// Wrapper around [JSContext] that provides lifetime information and convenient APIs.
pub struct Context<'c> {
	context: &'c mut *mut JSContext,
	rooted: RootedArena,
	local: LocalArena<'static>,
}

macro_rules! impl_root_methods {
	($(($fn_name:ident, $pointer:ty, $key:ident, $gc_type:ident)$(,)?)*) => {
		$(
			/// Roots a [$pointer], as a $gc_type, and returns a [Local] to it.
			pub fn $fn_name<'cx>(&'cx self, ptr: $pointer) -> Local<'cx, $pointer> {
				let rooted = self.rooted.$key.alloc(Rooted::new_unrooted());
				self.local.order.borrow_mut().push(GCType::$gc_type);
				Local::from_rooted(
					unsafe {
						transmute(self.local.$key.alloc(RootedGuard::new(*self.context, transmute(rooted), ptr)))
					}
				)
			}
		)*
	};
}

impl Context<'_> {
	/// Creates a new [Context] with a given lifetime.
	pub fn new(context: &mut *mut JSContext) -> Context {
		Context {
			context,
			rooted: RootedArena::default(),
			local: LocalArena::default(),
		}
	}

	impl_root_methods! {
		(root_value, JSVal, values, Value),
		(root_object, *mut JSObject, objects, Object),
		(root_string, *mut JSString, strings, String),
		(root_script, *mut JSScript, scripts, Script),
		(root_property_key, PropertyKey, property_keys, PropertyKey),
		(root_function, *mut JSFunction, functions, Function),
		(root_symbol, *mut Symbol, symbols, Symbol),
	}

	pub unsafe fn root_persistent_object(object: *mut JSObject) -> Local<'static, *mut JSObject> {
		let heap = *Heap::boxed(object);
		HEAP_OBJECTS.with(|persistent| {
			let mut persistent = persistent.borrow_mut();
			persistent.push(heap);
			let ptr = &persistent[persistent.len() - 1];
			RootedTraceableSet::add(ptr);
		});
		Local::from_marked(&object)
	}

	pub unsafe fn unroot_persistent_object(object: *mut JSObject) {
		HEAP_OBJECTS.with(|persistent| {
			let mut persistent = persistent.borrow_mut();
			let idx = match persistent.iter().rposition(|x| ptr::eq(x.get_unsafe() as *const _, object as *const _)) {
				Some(idx) => idx,
				None => return,
			};
			let heap = persistent.remove(idx);
			RootedTraceableSet::remove(&heap);
		});
	}
}

impl Deref for Context<'_> {
	type Target = *mut JSContext;

	fn deref(&self) -> &Self::Target {
		self.context
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
			(*mut JSFunction, functions, Function),
			(*mut Symbol, symbols, Symbol),
		}
	}
}
