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
	/// Represents the flags of properties on an [Object](crate::Object)
	pub struct PropertyFlags: u16 {
		/// Prevents enumeration through `Object.keys()`, `for...in` and other functions.
		/// See [Enumerability of Properties](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Enumerability_and_ownership_of_properties#traversing_object_properties) for more information
		const ENUMERATE = JSPROP_ENUMERATE as u16;
		/// Prevents reassignment of the property.
		const READ_ONLY = JSPROP_READONLY as u16;
		/// Prevents deletion and attribute modification of the property.
		const PERMANENT = JSPROP_PERMANENT as u16;
		const RESOLVING = JSPROP_RESOLVING as u16;

		const CONSTANT = PropertyFlags::READ_ONLY.bits | PropertyFlags::PERMANENT.bits;
		const CONSTANT_ENUMERATED = PropertyFlags::CONSTANT.bits | PropertyFlags::ENUMERATE.bits;
	}
}

bitflags! {
	/// Represents the flags when iterating over an [Object](crate::Object).
	pub struct IteratorFlags: u32 {
		/// Allows iterating over private properties.
		const PRIVATE = JSITER_PRIVATE;
		/// Disallows iterating over inherited properties.
		const OWN_ONLY = JSITER_OWNONLY;
		/// Allows iteration over non-enumerable properties.
		const HIDDEN = JSITER_HIDDEN;
		/// Allows iteration over symbol keys.
		const SYMBOLS = JSITER_SYMBOLS;
		/// Disallows iteration over string keys.
		const SYMBOLS_ONLY = JSITER_SYMBOLSONLY;
		/// Iteration over async iterable objects and async generators.
		const FOR_AWAIT_OF = JSITER_FORAWAITOF;
	}
}
