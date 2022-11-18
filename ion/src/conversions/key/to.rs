/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::rc::Rc;
use std::string::String as RustString;

use mozjs::jsapi::{JS_StringToId, JSString, PropertyKey};
use mozjs::jsapi::Symbol as JSSymbol;
use mozjs_sys::jsid::{IntId, SymbolId, VoidId};

use crate::{Context, Key, Local, String, Symbol};

pub trait ToKey<'cx> {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey>;
}

macro_rules! impl_into_key_for_integer {
	($ty:ty) => {
		impl<'cx> ToKey<'cx> for $ty {
			fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
				cx.root_property_key(IntId(*self as i32))
			}
		}
	};
}

impl_into_key_for_integer!(i8);
impl_into_key_for_integer!(i16);
impl_into_key_for_integer!(i32);

impl_into_key_for_integer!(u8);
impl_into_key_for_integer!(u16);
impl_into_key_for_integer!(u32);

impl<'cx> ToKey<'cx> for *mut JSString {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		String::from(cx.root_string(*self)).to_key(cx)
	}
}

impl<'cx> ToKey<'cx> for String<'cx> {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		let mut id = cx.root_property_key(VoidId());
		unsafe { JS_StringToId(**cx, self.handle().into(), id.handle_mut().into()) };
		id
	}
}

impl<'cx> ToKey<'cx> for RustString {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		String::new(cx, self).unwrap().to_key(cx)
	}
}

impl<'cx> ToKey<'cx> for str {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		String::new(cx, self).unwrap().to_key(cx)
	}
}

impl<'cx> ToKey<'cx> for *mut JSSymbol {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		cx.root_property_key(SymbolId(*self))
	}
}

impl<'cx> ToKey<'cx> for Symbol<'_> {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		cx.root_property_key(SymbolId(***self))
	}
}

impl<'cx> ToKey<'cx> for Key<'cx> {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		match self {
			Key::Int(i) => i.to_key(cx),
			Key::String(str) => str.to_key(cx),
			Key::Symbol(symbol) => symbol.to_key(cx),
			Key::Void => cx.root_property_key(VoidId()),
		}
	}
}

impl<'cx, K: ToKey<'cx>> ToKey<'cx> for &K {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		(*self).to_key(cx)
	}
}

impl<'cx, K: ToKey<'cx>> ToKey<'cx> for Box<K> {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		(**self).to_key(cx)
	}
}

impl<'cx, K: ToKey<'cx>> ToKey<'cx> for Rc<K> {
	fn to_key(&self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		(**self).to_key(cx)
	}
}
