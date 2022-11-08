/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::mem::transmute;
use std::ops::{Deref, DerefMut};

use mozjs::jsapi::{GetSymbolCode, GetSymbolDescription, GetSymbolFor, GetWellKnownSymbol, NewSymbol};
use mozjs::jsapi::Symbol as JSSymbol;
use mozjs::jsapi::SymbolCode as JSSymbolCode;

use crate::{Context, Local};
use crate::conversions::{FromValue, ToValue};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub enum WellKnownSymbolCode {
	IsConcatSpreadable,
	Iterator,
	Match,
	Replace,
	Search,
	Species,
	HasInstance,
	Split,
	ToPrimitive,
	ToStringTag,
	Unscopables,
	AsyncIterator,
	MatchAll,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum SymbolCode {
	WellKnown(WellKnownSymbolCode),
	PrivateNameSymbol,
	InSymbolRegistry,
	UniqueSymbol,
}

impl WellKnownSymbolCode {
	pub fn identifier(&self) -> &'static str {
		use WellKnownSymbolCode as WKSC;
		match self {
			WKSC::IsConcatSpreadable => "isConcatSpreadable",
			WKSC::Iterator => "iterator",
			WKSC::Match => "match",
			WKSC::Replace => "replace",
			WKSC::Search => "search",
			WKSC::Species => "species",
			WKSC::HasInstance => "hasInstance",
			WKSC::Split => "split",
			WKSC::ToPrimitive => "toPrimitive",
			WKSC::ToStringTag => "toStringTag",
			WKSC::Unscopables => "unscopables",
			WKSC::AsyncIterator => "asyncIterator",
			WKSC::MatchAll => "matchAll",
		}
	}
}

impl SymbolCode {
	pub fn well_known(&self) -> Option<WellKnownSymbolCode> {
		if let SymbolCode::WellKnown(code) = self {
			Some(*code)
		} else {
			None
		}
	}
}

impl From<JSSymbolCode> for SymbolCode {
	fn from(code: JSSymbolCode) -> SymbolCode {
		if (code as u32) < JSSymbolCode::Limit as u32 {
			SymbolCode::WellKnown(unsafe { transmute(code) })
		} else {
			use JSSymbolCode as JSSC;
			match code {
				JSSC::PrivateNameSymbol => SymbolCode::PrivateNameSymbol,
				JSSC::InSymbolRegistry => SymbolCode::InSymbolRegistry,
				JSSC::UniqueSymbol => SymbolCode::UniqueSymbol,
				_ => unreachable!(),
			}
		}
	}
}

impl From<WellKnownSymbolCode> for SymbolCode {
	fn from(code: WellKnownSymbolCode) -> SymbolCode {
		SymbolCode::WellKnown(code)
	}
}

impl From<WellKnownSymbolCode> for JSSymbolCode {
	fn from(code: WellKnownSymbolCode) -> Self {
		unsafe { transmute(code) }
	}
}

impl From<SymbolCode> for JSSymbolCode {
	fn from(code: SymbolCode) -> JSSymbolCode {
		use JSSymbolCode as JSSC;
		match code {
			SymbolCode::WellKnown(code) => code.into(),
			SymbolCode::PrivateNameSymbol => JSSC::PrivateNameSymbol,
			SymbolCode::InSymbolRegistry => JSSC::InSymbolRegistry,
			SymbolCode::UniqueSymbol => JSSC::UniqueSymbol,
		}
	}
}

#[derive(Debug)]
pub struct Symbol<'s> {
	symbol: Local<'s, *mut JSSymbol>,
}

impl<'s> Symbol<'s> {
	pub fn new<'cx>(cx: &'cx Context, description: &str) -> Symbol<'cx> {
		let description = unsafe { description.as_value(cx) };
		let description = cx.root_string(description.to_string());

		let symbol = unsafe { NewSymbol(**cx, description.handle().into()) };
		Symbol { symbol: cx.root_symbol(symbol) }
	}

	pub fn for_key<'cx>(cx: &'cx Context, key: &str) -> Symbol<'cx> {
		let key = unsafe { key.as_value(cx) };
		let key = cx.root_string(key.to_string());

		let symbol = unsafe { GetSymbolFor(**cx, key.handle().into()) };
		Symbol { symbol: cx.root_symbol(symbol) }
	}

	pub fn well_known<'cx>(cx: &'cx Context, code: WellKnownSymbolCode) -> Symbol<'cx> {
		let symbol = unsafe { GetWellKnownSymbol(**cx, code.into()) };
		Symbol { symbol: cx.root_symbol(symbol) }
	}

	pub fn code(&self) -> SymbolCode {
		unsafe { GetSymbolCode(self.symbol.handle().into()).into() }
	}

	pub fn description(&self, cx: &Context) -> Option<String> {
		let description = unsafe { GetSymbolDescription(self.symbol.handle().into()) };
		if !description.is_null() {
			unsafe {
				let description = description.as_value(cx);
				String::from_value(cx, &description, true, ()).ok()
			}
		} else {
			None
		}
	}
}

impl<'o> From<Local<'o, *mut JSSymbol>> for Symbol<'o> {
	fn from(symbol: Local<'o, *mut JSSymbol>) -> Symbol<'o> {
		Symbol { symbol }
	}
}

impl<'o> Deref for Symbol<'o> {
	type Target = Local<'o, *mut JSSymbol>;

	fn deref(&self) -> &Self::Target {
		&self.symbol
	}
}

impl<'o> DerefMut for Symbol<'o> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.symbol
	}
}

#[cfg(test)]
mod tests {
	use mozjs::jsapi::SymbolCode as JSSymbolCode;
	use crate::symbol::{SymbolCode, WellKnownSymbolCode};

	macro_rules! convert_codes {
		($(($js:expr, $native:expr)$(,)?)*) => {
			$(
				assert_eq!($js, JSSymbolCode::from($native));
				assert_eq!($native, SymbolCode::from($js));
			)*
		}
	}

	#[test]
	fn code_conversion() {
		use JSSymbolCode as JSSC;
		use SymbolCode as SC;
		use WellKnownSymbolCode as WKSC;

		// Well Known Symbol Codes
		convert_codes! {
			(JSSC::isConcatSpreadable, SC::WellKnown(WKSC::IsConcatSpreadable)),
			(JSSC::iterator, SC::WellKnown(WKSC::Iterator)),
			(JSSC::match_, SC::WellKnown(WKSC::Match)),
			(JSSC::replace, SC::WellKnown(WKSC::Replace)),
			(JSSC::search, SC::WellKnown(WKSC::Search)),
			(JSSC::species, SC::WellKnown(WKSC::Species)),
			(JSSC::hasInstance, SC::WellKnown(WKSC::HasInstance)),
			(JSSC::toPrimitive, SC::WellKnown(WKSC::ToPrimitive)),
			(JSSC::toStringTag, SC::WellKnown(WKSC::ToStringTag)),
			(JSSC::unscopables, SC::WellKnown(WKSC::Unscopables)),
			(JSSC::asyncIterator, SC::WellKnown(WKSC::AsyncIterator)),
			(JSSC::matchAll, SC::WellKnown(WKSC::MatchAll)),
		}

		// Other Symbol Codes
		convert_codes! {
			(JSSC::PrivateNameSymbol, SC::PrivateNameSymbol),
			(JSSC::InSymbolRegistry, SC::InSymbolRegistry),
			(JSSC::UniqueSymbol, SC::UniqueSymbol),
		}
	}
}
