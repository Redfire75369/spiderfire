/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;

use quote::ToTokens;
use syn::{DeriveInput, Error, ItemFn, ItemMod};

use crate::class::impl_js_class;
use crate::function::impl_js_fn;
use crate::jsval::impl_from_jsval;

pub(crate) mod class;
pub(crate) mod function;
pub(crate) mod jsval;
pub(crate) mod utils;

#[proc_macro_attribute]
pub fn js_fn(_attr: TokenStream, stream: TokenStream) -> TokenStream {
	let function = parse_macro_input!(stream as ItemFn);

	match impl_js_fn(function) {
		Ok(function) => function.to_token_stream().into(),
		Err(error) => error.to_compile_error().into(),
	}
}

#[proc_macro_attribute]
pub fn js_class(_attr: TokenStream, stream: TokenStream) -> TokenStream {
	let module = parse_macro_input!(stream as ItemMod);
	if module.content.is_none() {
		let error = Error::new_spanned(module, "Expected Non-Empty Module");
		return TokenStream::from(error.to_compile_error());
	}

	match impl_js_class(module) {
		Ok(module) => module.to_token_stream().into(),
		Err(error) => error.to_compile_error().into(),
	}
}

#[proc_macro_derive(FromJSVal, attributes(ion))]
pub fn from_jsval(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	match impl_from_jsval(input) {
		Ok(to_jsval) => to_jsval.to_token_stream().into(),
		Err(error) => error.to_compile_error().into(),
	}
}
