/*c
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::mem::{take, transmute};
use std::ops::Deref;

use mozjs::jsapi::{JSContext, JSFunction, JSObject, JSScript, JSString, PropertyKey, Rooted};
use mozjs::jsval::JSVal;
use mozjs::rust::RootedGuard;
use typed_arena::Arena;

use crate::Local;

#[allow(dead_code)]
pub enum GCType {
	Value,
	Object,
	String,
	Script,
	PropertyKey,
	Function,
	// TODO: Add Symbol Rooting
	// TOOD: Waiting on https://github.com/servo/mozjs/issues/306
	Symbol,
}

#[derive(Default)]
struct RootedArena {
	values: Arena<Rooted<JSVal>>,
	objects: Arena<Rooted<*mut JSObject>>,
	strings: Arena<Rooted<*mut JSString>>,
	scripts: Arena<Rooted<*mut JSScript>>,
	property_keys: Arena<Rooted<PropertyKey>>,
	functions: Arena<Rooted<*mut JSFunction>>,
}

#[derive(Default)]
struct LocalArena<'a> {
	order: RefCell<Vec<GCType>>,
	values: Arena<Local<'a, JSVal>>,
	objects: Arena<Local<'a, *mut JSObject>>,
	strings: Arena<Local<'a, *mut JSString>>,
	scripts: Arena<Local<'a, *mut JSScript>>,
	property_keys: Arena<Local<'a, PropertyKey>>,
	functions: Arena<Local<'a, *mut JSFunction>>,
}

pub struct Context<'c> {
	context: &'c mut *mut JSContext,
	rooted: RootedArena,
	local: LocalArena<'static>,
}

macro_rules! impl_root_method {
	($(($fn_name:ident, $pointer:ty, $key:ident, $gc_type:ident)$(,)?)*) => {
		$(
			pub fn $fn_name<'cx>(&'cx self, ptr: $pointer) -> &'cx mut Local<'cx, $pointer> {
				let rooted = self.rooted.$key.alloc(Rooted::new_unrooted());
				self.local.order.borrow_mut().push(GCType::$gc_type);
				unsafe {
					transmute(self.local.$key.alloc(
						Local::from_rooted(RootedGuard::new(*self.context, transmute(rooted), ptr))
					))
				}
			}
		)*
	};
}

impl Context<'_> {
	pub fn new(context: &mut *mut JSContext) -> Context {
		Context {
			context,
			rooted: RootedArena::default(),
			local: LocalArena::default(),
		}
	}

	impl_root_method! {
		(root_value, JSVal, values, Value),
		(root_object, *mut JSObject, objects, Object),
		(root_string, *mut JSString, strings, String),
		(root_script, *mut JSScript, scripts, Script),
		(root_property_key, PropertyKey, property_keys, PropertyKey),
		(root_function, *mut JSFunction, functions, Function),
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
				GCType::Symbol => (),
			}
		}
	}
}

impl Drop for Context<'_> {
	fn drop(&mut self) {
		impl_drop! {
			[self],
			(JSVal, values, Value),
			(*mut JSObject, objects, Object),
			(*mut JSString, strings, String),
			(*mut JSScript, scripts, Script),
			(PropertyKey, property_keys, PropertyKey),
			(*mut JSFunction, functions, Function),
		}
	}
}
