/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::rc::Rc;
use std::string::String as RustString;

use mozjs::jsapi::{JS_StringToId, JS_ValueToId, JSString};
use mozjs::jsapi::PropertyKey as JSPropertyKey;
use mozjs::jsapi::Symbol as JSSymbol;
use mozjs::jsid::{SymbolId, VoidId};
use mozjs::jsval::JSVal;

use crate::{Context, OwnedKey, PropertyKey, String, Symbol, Value};
use crate::symbol::WellKnownSymbolCode;

/// Represents types that can be converted to [property keys](PropertyKey).
pub trait ToPropertyKey {
	/// Converts `self` to a new [`PropertyKey`](PropertyKey).
	/// Returns `None` when conversion fails.
	fn to_key(&self, cx: &Context) -> Option<PropertyKey>;
}

macro_rules! impl_to_key_for_integer {
	($ty:ty) => {
		impl ToPropertyKey for $ty {
			fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
				Some(PropertyKey::with_int(cx, *self as i32))
			}
		}
	};
}

impl_to_key_for_integer!(i8);
impl_to_key_for_integer!(i16);
impl_to_key_for_integer!(i32);

impl_to_key_for_integer!(u8);
impl_to_key_for_integer!(u16);
impl_to_key_for_integer!(u32);

impl ToPropertyKey for *mut JSString {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		String::from(cx.root(*self)).to_key(cx)
	}
}

impl ToPropertyKey for String {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		rooted!(in(cx.as_ptr()) let mut key = VoidId());
		if unsafe { JS_StringToId(cx.as_ptr(), self.handle().into(), key.handle_mut().into()) } {
			Some(PropertyKey::from(cx.root(key.get())))
		} else {
			None
		}
	}
}

impl ToPropertyKey for RustString {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		String::copy_from_str(cx, self)?.to_key(cx)
	}
}

impl ToPropertyKey for &str {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		String::copy_from_str(cx, self)?.to_key(cx)
	}
}

impl ToPropertyKey for *mut JSSymbol {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		Some(cx.root(SymbolId(*self)).into())
	}
}

impl ToPropertyKey for Symbol {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		self.handle().to_key(cx)
	}
}

impl ToPropertyKey for WellKnownSymbolCode {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		Symbol::well_known(cx, *self).to_key(cx)
	}
}

impl ToPropertyKey for JSVal {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		Value::from(cx.root(*self)).to_key(cx)
	}
}

impl ToPropertyKey for Value {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		rooted!(in(cx.as_ptr()) let mut key = VoidId());
		if unsafe { JS_ValueToId(cx.as_ptr(), self.handle().into(), key.handle_mut().into()) } {
			Some(PropertyKey::from(cx.root(key.get())))
		} else {
			None
		}
	}
}

impl ToPropertyKey for JSPropertyKey {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		Some(cx.root(*self).into())
	}
}

impl ToPropertyKey for PropertyKey {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		self.handle().to_key(cx)
	}
}

impl ToPropertyKey for OwnedKey {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		match self {
			OwnedKey::Int(i) => i.to_key(cx),
			OwnedKey::String(str) => str.to_key(cx),
			OwnedKey::Symbol(symbol) => symbol.to_key(cx),
			OwnedKey::Void => Some(cx.root(VoidId()).into()),
		}
	}
}

impl<K: ToPropertyKey> ToPropertyKey for &K {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		(**self).to_key(cx)
	}
}

impl<K: ToPropertyKey> ToPropertyKey for Box<K> {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		(**self).to_key(cx)
	}
}

impl<K: ToPropertyKey> ToPropertyKey for Rc<K> {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		(**self).to_key(cx)
	}
}

impl<K: ToPropertyKey> ToPropertyKey for Option<K> {
	fn to_key(&self, cx: &Context) -> Option<PropertyKey> {
		self.as_ref().and_then(|k| k.to_key(cx))
	}
}
