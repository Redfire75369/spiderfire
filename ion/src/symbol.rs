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

#[derive(Copy, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
pub enum SymbolCode {
	WellKnown(WellKnownSymbolCode),
	PrivateNameSymbol,
	InSymbolRegistry,
	UniqueSymbol,
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

	pub unsafe fn code(&self) -> SymbolCode {
		GetSymbolCode(self.symbol.handle().into()).into()
	}

	pub unsafe fn description(&self, cx: &Context) -> Option<String> {
		let description = GetSymbolDescription(self.symbol.handle().into());
		if !description.is_null() {
			let description = description.as_value(cx);
			String::from_value(cx, &description, true, ()).ok()
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
