/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use mozjs::jsapi::JSClass;

pub const MAX_PROTO_CHAIN_LENGTH: usize = 8;

pub trait TypeIdWrap: private::Sealed + 'static {
	fn type_id(&self) -> TypeId;
}

mod private {
	pub trait Sealed {}

	impl<T: 'static> Sealed for super::TypeIdWrapper<T> {}
}

pub struct TypeIdWrapper<T: 'static> {
	_private: PhantomData<T>,
}

impl<T: 'static> TypeIdWrapper<T> {
	pub const fn new() -> TypeIdWrapper<T> {
		TypeIdWrapper { _private: PhantomData }
	}
}

impl<T: 'static> TypeIdWrap for TypeIdWrapper<T> {
	fn type_id(&self) -> TypeId {
		TypeId::of::<T>()
	}
}

unsafe impl<T: 'static> Send for TypeIdWrapper<T> {}

unsafe impl<T: 'static> Sync for TypeIdWrapper<T> {}

pub type PrototypeChain = [Option<&'static (dyn TypeIdWrap + Send + Sync)>; MAX_PROTO_CHAIN_LENGTH];

#[repr(C)]
pub struct NativeClass {
	pub base: JSClass,
	pub prototype_chain: PrototypeChain,
}

impl Debug for NativeClass {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("NativeClass")
			.field("base", &self.base)
			.field(
				"prototype_chain",
				&format!("[TypeIdWrapper; {}]", self.prototype_chain.iter().filter(|proto| proto.is_some()).count()),
			)
			.finish()
	}
}
