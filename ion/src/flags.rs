/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::{
	JSITER_FORAWAITOF, JSITER_HIDDEN, JSITER_OWNONLY, JSITER_PRIVATE, JSITER_SYMBOLS, JSITER_SYMBOLSONLY, JSPROP_ENUMERATE, JSPROP_PERMANENT,
	JSPROP_READONLY, JSPROP_RESOLVING,
};

bitflags! {
	pub struct PropertyFlags: u16 {
		const ENUMERATE = JSPROP_ENUMERATE as u16;
		const READ_ONLY = JSPROP_READONLY as u16;
		const PERMANENT = JSPROP_PERMANENT as u16;
		const RESOLVING = JSPROP_RESOLVING as u16;

		const CONSTANT = PropertyFlags::READ_ONLY.bits | PropertyFlags::PERMANENT.bits;
		const CONSTANT_ENUMERATED = PropertyFlags::CONSTANT.bits | PropertyFlags::ENUMERATE.bits;
	}
}

bitflags! {
	pub struct IteratorFlags: u32 {
		const PRIVATE = JSITER_PRIVATE;
		const OWN_ONLY = JSITER_OWNONLY;
		const HIDDEN = JSITER_HIDDEN;
		const SYMBOLS = JSITER_SYMBOLS;
		const SYMBOLS_ONLY = JSITER_SYMBOLSONLY;
		const FOR_AWAIT_OF = JSITER_FORAWAITOF;
	}
}
