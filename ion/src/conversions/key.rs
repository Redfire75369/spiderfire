/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::rc::Rc;
use std::string::String as RustString;

use mozjs::jsapi::{JS_StringToId, JSString};
use mozjs::jsapi::PropertyKey as JSPropertyKey;
use mozjs::jsapi::Symbol as JSSymbol;
use mozjs::jsid::{SymbolId, VoidId};
use mozjs_sys::jsapi::JS_ValueToId;
use mozjs_sys::jsval::JSVal;

use crate::{Context, OwnedKey, PropertyKey, String, Symbol, Value};

pub trait ToKey<'cx> {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>>;
}

macro_rules! impl_to_key_for_integer {
	($ty:ty) => {
		impl<'cx> ToKey<'cx> for $ty {
			fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
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

impl<'cx> ToKey<'cx> for *mut JSString {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		String::from(cx.root_string(*self)).to_key(cx)
	}
}

impl<'cx> ToKey<'cx> for String<'cx> {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		let mut key = PropertyKey::from(cx.root_property_key(VoidId()));
		(unsafe { JS_StringToId(**cx, self.handle().into(), key.handle_mut().into()) }).then(|| key)
	}
}

impl<'cx> ToKey<'cx> for RustString {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		String::new(cx, &self)?.to_key(cx)
	}
}

impl<'cx> ToKey<'cx> for &str {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		String::new(cx, self)?.to_key(cx)
	}
}

impl<'cx> ToKey<'cx> for *mut JSSymbol {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		Some(cx.root_property_key(SymbolId(*self)).into())
	}
}

impl<'cx> ToKey<'cx> for Symbol<'_> {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		(**self).to_key(cx)
	}
}

impl<'cx> ToKey<'cx> for JSVal {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		Value::from(cx.root_value(*self)).to_key(cx)
	}
}

impl<'cx> ToKey<'cx> for Value<'cx> {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		let mut key = PropertyKey::from(cx.root_property_key(VoidId()));
		(unsafe { JS_ValueToId(**cx, self.handle().into(), key.handle_mut().into()) }).then(|| key)
	}
}

impl<'cx> ToKey<'cx> for JSPropertyKey {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		Some(cx.root_property_key(*self).into())
	}
}

impl<'cx> ToKey<'cx> for PropertyKey<'cx> {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		(**self).to_key(cx)
	}
}

impl<'cx> ToKey<'cx> for OwnedKey<'cx> {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		match self {
			OwnedKey::Int(i) => i.to_key(cx),
			OwnedKey::String(str) => str.to_key(cx),
			OwnedKey::Symbol(symbol) => symbol.to_key(cx),
			OwnedKey::Void => Some(cx.root_property_key(VoidId()).into()),
		}
	}
}

impl<'cx, K: ToKey<'cx>> ToKey<'cx> for &K {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		(**self).to_key(cx)
	}
}

impl<'cx, K: ToKey<'cx>> ToKey<'cx> for Box<K> {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		(**self).to_key(cx)
	}
}

impl<'cx, K: ToKey<'cx>> ToKey<'cx> for Rc<K> {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		(**self).to_key(cx)
	}
}

impl<'cx, K: ToKey<'cx>> ToKey<'cx> for Option<K> {
	fn to_key(&self, cx: &'cx Context) -> Option<PropertyKey<'cx>> {
		self.as_ref().and_then(|k| k.to_key(cx))
	}
}
