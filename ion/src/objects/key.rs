/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::hash::{Hash, Hasher};
use std::mem::discriminant;
use std::ops::{Deref, DerefMut};

use mozjs::jsapi::{Heap, JS_IdToProtoKey, JSProtoKey, ProtoKeyToId};
use mozjs::jsapi::PropertyKey as JSPropertyKey;
use mozjs::jsid::{IntId, VoidId};

use crate::{Context, Root, String, Symbol, Value};
use crate::conversions::ToPropertyKey;

pub struct PropertyKey {
	key: Root<Box<Heap<JSPropertyKey>>>,
}

impl PropertyKey {
	/// Creates a [PropertyKey] from an integer.
	pub fn with_int(cx: &Context, int: i32) -> PropertyKey {
		PropertyKey::from(cx.root_property_key(IntId(int)))
	}

	/// Creates a [PropertyKey] from a string.
	pub fn with_string(cx: &Context, string: &str) -> Option<PropertyKey> {
		let string = String::copy_from_str(cx, string)?;
		string.to_key(cx)
	}

	pub fn with_symbol(cx: &Context, symbol: &Symbol) -> PropertyKey {
		symbol.to_key(cx).unwrap()
	}

	pub fn from_proto_key(cx: &Context, proto_key: JSProtoKey) -> PropertyKey {
		rooted!(in(cx.as_ptr()) let mut key = VoidId());
		unsafe { ProtoKeyToId(cx.as_ptr(), proto_key, key.handle_mut().into()) }
		PropertyKey::from(cx.root(key.get()))
	}

	pub fn from_value(cx: &Context, value: &Value) -> Option<PropertyKey> {
		value.to_key(cx)
	}

	pub fn to_proto_key(&self, cx: &Context) -> Option<JSProtoKey> {
		let proto_key = unsafe { JS_IdToProtoKey(cx.as_ptr(), self.handle().into()) };
		(proto_key != JSProtoKey::JSProto_Null).then_some(proto_key)
	}

	pub fn to_owned_key(&self, cx: &Context) -> OwnedKey {
		if self.handle().is_int() {
			OwnedKey::Int(self.handle().to_int())
		} else if self.handle().is_string() {
			OwnedKey::String(String::from(cx.root_string(self.handle().to_string())).to_owned(cx))
		} else if self.handle().is_symbol() {
			OwnedKey::Symbol(cx.root_symbol(self.handle().to_symbol()).into())
		} else {
			OwnedKey::Void
		}
	}

	pub fn into_root(self) -> Root<Box<Heap<JSPropertyKey>>> {
		self.key
	}
}

impl From<Root<Box<Heap<JSPropertyKey>>>> for PropertyKey {
	fn from(key: Root<Box<Heap<JSPropertyKey>>>) -> PropertyKey {
		PropertyKey { key }
	}
}

impl Deref for PropertyKey {
	type Target = Root<Box<Heap<JSPropertyKey>>>;

	fn deref(&self) -> &Self::Target {
		&self.key
	}
}

impl DerefMut for PropertyKey {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.key
	}
}

/// Represents the key on an object.
#[derive(Debug)]
pub enum OwnedKey {
	Int(i32),
	String(std::string::String),
	Symbol(Symbol),
	Void,
}

impl OwnedKey {
	pub fn clone(&self, cx: &Context) -> OwnedKey {
		match self {
			OwnedKey::Int(i) => OwnedKey::Int(*i),
			OwnedKey::String(str) => OwnedKey::String(str.clone()),
			OwnedKey::Symbol(symbol) => OwnedKey::Symbol(cx.root_symbol(symbol.get()).into()),
			OwnedKey::Void => OwnedKey::Void,
		}
	}
}

impl Hash for OwnedKey {
	fn hash<H: Hasher>(&self, state: &mut H) {
		discriminant(self).hash(state);
		match self {
			OwnedKey::Int(i) => i.hash(state),
			OwnedKey::String(str) => str.hash(state),
			OwnedKey::Symbol(symbol) => symbol.handle().hash(state),
			OwnedKey::Void => (),
		}
	}
}

impl PartialEq for OwnedKey {
	fn eq(&self, other: &OwnedKey) -> bool {
		match (self, other) {
			(OwnedKey::Int(i), OwnedKey::Int(i2)) => *i == *i2,
			(OwnedKey::String(str), OwnedKey::String(str2)) => *str == *str2,
			(OwnedKey::Symbol(symbol), OwnedKey::Symbol(symbol2)) => symbol.get() == symbol2.get(),
			(OwnedKey::Void, OwnedKey::Void) => true,
			_ => false,
		}
	}
}

impl Eq for OwnedKey {}
