/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::hash::{Hash, Hasher};
use std::mem::discriminant;
use std::ops::Deref;

use mozjs::jsapi::PropertyKey as JSPropertyKey;
use mozjs::jsid::{IntId, VoidId};
use mozjs::rust::{Handle, MutableHandle};
use mozjs_sys::jsapi::{JS_IdToProtoKey, JS_ValueToId, JSProtoKey};
use mozjs_sys::jsapi::JS::ProtoKeyToId;

use crate::{Context, Local, String, Symbol, Value};
use crate::conversions::ToPropertyKey;

pub struct PropertyKey<'k> {
	key: Local<'k, JSPropertyKey>,
}

impl<'k> PropertyKey<'k> {
	/// Creates a [PropertyKey] from an integer.
	pub fn with_int<'cx>(cx: &'cx Context, int: i32) -> PropertyKey<'cx> {
		PropertyKey::from(cx.root_property_key(IntId(int)))
	}

	/// Creates a [PropertyKey] from a string.
	pub fn with_string<'cx>(cx: &'cx Context, string: &str) -> Option<PropertyKey<'cx>> {
		let string = String::new(cx, string)?;
		string.to_key(cx)
	}

	pub fn with_symbol<'cx: 's, 's>(cx: &'cx Context, symbol: &Symbol<'s>) -> PropertyKey<'cx> {
		symbol.to_key(cx).unwrap()
	}

	pub fn from_proto_key<'cx>(cx: &'cx Context, proto_key: JSProtoKey) -> PropertyKey<'cx> {
		let mut key = PropertyKey::from(cx.root_property_key(VoidId()));
		unsafe { ProtoKeyToId(**cx, proto_key, key.handle_mut().into()) }
		key
	}

	pub fn from_value<'cx>(cx: &'cx Context, value: &Value<'cx>) -> Option<PropertyKey<'cx>> {
		let mut key = PropertyKey::from(cx.root_property_key(VoidId()));
		(unsafe { JS_ValueToId(**cx, value.handle().into(), key.handle_mut().into()) }).then_some(key)
	}

	pub fn to_proto_key(&self, cx: &Context) -> Option<JSProtoKey> {
		let proto_key = unsafe { JS_IdToProtoKey(**cx, self.handle().into()) };
		(proto_key != JSProtoKey::JSProto_Null).then_some(proto_key)
	}

	pub fn to_owned_key<'cx>(&self, cx: &'cx Context) -> OwnedKey<'cx> {
		if self.key.is_int() {
			OwnedKey::Int(self.key.to_int())
		} else if self.key.is_string() {
			OwnedKey::String(String::from(cx.root_string(self.key.to_string())).to_owned(cx).unwrap())
		} else if self.key.is_symbol() {
			OwnedKey::Symbol(cx.root_symbol(self.to_symbol()).into())
		} else {
			OwnedKey::Void
		}
	}

	pub fn handle<'s>(&'s self) -> Handle<'s, JSPropertyKey>
	where
		'k: 's,
	{
		self.key.handle()
	}

	pub fn handle_mut<'s>(&'s mut self) -> MutableHandle<'s, JSPropertyKey>
	where
		'k: 's,
	{
		self.key.handle_mut()
	}

	pub fn into_local(self) -> Local<'k, JSPropertyKey> {
		self.key
	}
}

impl<'k> From<Local<'k, JSPropertyKey>> for PropertyKey<'k> {
	fn from(key: Local<'k, JSPropertyKey>) -> PropertyKey<'k> {
		PropertyKey { key }
	}
}

impl<'k> Deref for PropertyKey<'k> {
	type Target = Local<'k, JSPropertyKey>;

	fn deref(&self) -> &Local<'k, JSPropertyKey> {
		&self.key
	}
}

/// Represents the key on an object.
#[derive(Debug)]
pub enum OwnedKey<'k> {
	Int(i32),
	String(std::string::String),
	Symbol(Symbol<'k>),
	Void,
}

impl<'k> OwnedKey<'k> {
	pub fn clone(&self, cx: &'k Context) -> OwnedKey<'k> {
		match self {
			OwnedKey::Int(i) => OwnedKey::Int(*i),
			OwnedKey::String(str) => OwnedKey::String(str.clone()),
			OwnedKey::Symbol(symbol) => OwnedKey::Symbol(cx.root_symbol(***symbol).into()),
			OwnedKey::Void => OwnedKey::Void,
		}
	}
}

impl Hash for OwnedKey<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		discriminant(self).hash(state);
		match self {
			OwnedKey::Int(i) => i.hash(state),
			OwnedKey::String(str) => str.hash(state),
			OwnedKey::Symbol(symbol) => symbol.hash(state),
			OwnedKey::Void => (),
		}
	}
}

impl PartialEq for OwnedKey<'_> {
	fn eq(&self, other: &OwnedKey<'_>) -> bool {
		match (self, other) {
			(OwnedKey::Int(i), OwnedKey::Int(i2)) => *i == *i2,
			(OwnedKey::String(str), OwnedKey::String(str2)) => *str == *str2,
			(OwnedKey::Symbol(symbol), OwnedKey::Symbol(symbol2)) => ***symbol == ***symbol2,
			(OwnedKey::Void, OwnedKey::Void) => true,
			_ => false,
		}
	}
}

impl Eq for OwnedKey<'_> {}
