/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::hash::{Hash, Hasher};
use std::mem::discriminant;

use crate::Symbol;

#[derive(Debug)]
pub enum Key<'k> {
	Int(i32),
	String(String),
	Symbol(Symbol<'k>),
	Void,
}

impl Hash for Key<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		discriminant(self).hash(state);
		match self {
			Key::Int(i) => i.hash(state),
			Key::String(str) => str.hash(state),
			Key::Symbol(symbol) => symbol.hash(state),
			Key::Void => (),
		}
	}
}

impl PartialEq for Key<'_> {
	fn eq(&self, other: &Key<'_>) -> bool {
		match (self, other) {
			(Key::Int(i), Key::Int(i2)) => *i == *i2,
			(Key::String(str), Key::String(str2)) => *str == *str2,
			(Key::Symbol(symbol), Key::Symbol(symbol2)) => ***symbol == ***symbol2,
			(Key::Void, Key::Void) => true,
			_ => false,
		}
	}
}

impl Eq for Key<'_> {}
