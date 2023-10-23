/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{Attribute, Result};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Crate;

mod keywords {
	custom_keyword!(runtime);
}

#[allow(dead_code)]
pub(crate) enum CrateAttribute {
	Ion { kw: Crate, eq: Token![=], krate: TokenStream },
	Runtime { kw: keywords::runtime, eq: Token![=], krate: TokenStream },
}

impl Parse for CrateAttribute {
	fn parse(input: ParseStream) -> Result<CrateAttribute> {
		use CrateAttribute as CA;

		let lookahead = input.lookahead1();
		if lookahead.peek(Crate) {
			Ok(CA::Ion {
				kw: input.parse()?,
				eq: input.parse()?,
				krate: input.parse()?,
			})
		} else if lookahead.peek(keywords::runtime) {
			Ok(CA::Runtime {
				kw: input.parse()?,
				eq: input.parse()?,
				krate: input.parse()?,
			})
		} else {
			Err(lookahead.error())
		}
	}
}

#[derive(Debug)]
pub(crate) struct Crates {
	pub(crate) ion: TokenStream,
	pub(crate) runtime: TokenStream,
}

impl Crates {
	pub(crate) fn from_attributes(attrs: &[Attribute]) -> Crates {
		let mut ion = None;
		let mut runtime = None;

		for attr in attrs {
			if attr.path().is_ident("ion") {
				let args: Punctuated<CrateAttribute, Token![,]> = match attr.parse_args_with(Punctuated::parse_terminated) {
					Ok(p) => p,
					Err(_) => continue,
				};
				for arg in args {
					match arg {
						CrateAttribute::Ion { krate, .. } => ion = Some(krate),
						CrateAttribute::Runtime { krate, .. } => runtime = Some(krate),
					}
				}
			}
		}

		Crates {
			ion: ion.unwrap_or_else(|| parse_quote!(::ion)),
			runtime: runtime.unwrap_or_else(|| parse_quote!(::runtime)),
		}
	}
}
