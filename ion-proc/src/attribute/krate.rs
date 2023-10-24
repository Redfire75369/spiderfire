/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{Attribute, Result};
use syn::parse::{Parse, ParseStream};

#[allow(dead_code)]
struct CrateAttribute {
	kw: Token![crate],
	eq: Token![=],
	krate: TokenStream,
}

impl Parse for CrateAttribute {
	fn parse(input: ParseStream) -> Result<CrateAttribute> {
		let lookahead = input.lookahead1();
		if lookahead.peek(Token![crate]) {
			Ok(CrateAttribute {
				kw: input.parse()?,
				eq: input.parse()?,
				krate: input.parse()?,
			})
		} else {
			Err(lookahead.error())
		}
	}
}

pub fn crate_from_attributes(attrs: &[Attribute]) -> TokenStream {
	for attr in attrs {
		if attr.path().is_ident("ion") {
			if let Ok(CrateAttribute { krate, .. }) = attr.parse_args::<CrateAttribute>() {
				return krate;
			}
		}
	}

	parse_quote!(::ion)
}
