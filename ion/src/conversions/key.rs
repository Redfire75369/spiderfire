/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::string::String as RustString;

use mozjs::jsapi::{JS_StringToId, JSString, PropertyKey};
use mozjs::jsapi::Symbol as JSSymbol;
use mozjs::jsid::{IntId, SymbolId, VoidId};

use crate::{Context, Key, Local, String, Symbol};

pub trait IntoKey<'cx> {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey>;
}

macro_rules! impl_into_key_for_integer {
	($ty:ty) => {
		impl<'cx> IntoKey<'cx> for $ty {
			fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
				cx.root_property_key(IntId(self as i32))
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

impl<'cx> IntoKey<'cx> for *mut JSString {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		String::from(cx.root_string(self)).into_key(cx)
	}
}

impl<'cx> IntoKey<'cx> for String<'cx> {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		let mut id = cx.root_property_key(VoidId());
		unsafe { JS_StringToId(**cx, self.handle().into(), id.handle_mut().into()) };
		id
	}
}

impl<'cx> IntoKey<'cx> for RustString {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		String::new(cx, &self).unwrap().into_key(cx)
	}
}

impl<'cx> IntoKey<'cx> for &str {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		String::new(cx, self).unwrap().into_key(cx)
	}
}

impl<'cx> IntoKey<'cx> for *mut JSSymbol {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		cx.root_property_key(SymbolId(self))
	}
}

impl<'cx> IntoKey<'cx> for Symbol<'_> {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		cx.root_property_key(SymbolId(**self))
	}
}

impl<'cx> IntoKey<'cx> for Key<'cx> {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		match self {
			Key::Int(i) => i.into_key(cx),
			Key::String(str) => str.into_key(cx),
			Key::Symbol(symbol) => symbol.into_key(cx),
			Key::Void => cx.root_property_key(VoidId()),
		}
	}
}

impl<'cx> IntoKey<'cx> for &Key<'cx> {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		match self {
			Key::Int(i) => (*i).into_key(cx),
			Key::String(str) => str.clone().into_key(cx),
			Key::Symbol(symbol) => (***symbol).into_key(cx),
			Key::Void => cx.root_property_key(VoidId()),
		}
	}
}

impl<'cx> IntoKey<'cx> for Local<'cx, PropertyKey> {
	fn into_key(self, _: &'cx Context) -> Local<'cx, PropertyKey> {
		self
	}
}

impl<'cx, K: IntoKey<'cx> + Clone> IntoKey<'cx> for &K {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		self.clone().into_key(cx)
	}
}

impl<'cx, K: IntoKey<'cx>> IntoKey<'cx> for Box<K> {
	fn into_key(self, cx: &'cx Context) -> Local<'cx, PropertyKey> {
		(*self).into_key(cx)
	}
}
