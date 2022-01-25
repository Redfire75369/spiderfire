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
use syn::ItemFn;

use crate::function::impl_js_fn;

mod function;

#[proc_macro_attribute]
pub fn js_fn(_attr: TokenStream, stream: TokenStream) -> TokenStream {
	let function = parse_macro_input!(stream as ItemFn);

	match impl_js_fn(function) {
		Ok(function) => TokenStream::from(function.to_token_stream()),
		Err(error) => TokenStream::from(error.to_compile_error()),
	}
}
