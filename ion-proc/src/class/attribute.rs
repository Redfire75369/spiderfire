/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use syn::{Expr, Result};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Bracket;

use crate::class::method::MethodKind;

mod keywords {
	custom_keyword!(alias);

	custom_keyword!(convert);

	custom_keyword!(constructor);
	custom_keyword!(get);
	custom_keyword!(set);
	custom_keyword!(internal);
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Aliases {
	pub(crate) alias: keywords::alias,
	pub(crate) eq: Token![=],
	pub(crate) bracket: Bracket,
	pub(crate) aliases: Punctuated<Ident, Token![,]>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum PropertyAttribute {
	Convert {
		convert: keywords::convert,
		eq: Token![=],
		conversion: Box<Expr>,
	},
	Alias(Aliases),
}

#[derive(Debug)]
pub(crate) enum MethodAttribute {
	Constructor(keywords::constructor),
	Getter(keywords::get),
	Setter(keywords::set),
	Internal(keywords::internal),
	Alias(Aliases),
}

impl MethodAttribute {
	pub(crate) fn to_kind(&self) -> Option<MethodKind> {
		use MethodAttribute::*;
		match self {
			Constructor(_) => Some(MethodKind::Constructor),
			Getter(_) => Some(MethodKind::Getter),
			Setter(_) => Some(MethodKind::Setter),
			Internal(_) => Some(MethodKind::Internal),
			Alias { .. } => None,
		}
	}
}

impl Parse for Aliases {
	fn parse(input: ParseStream) -> Result<Aliases> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::alias) {
			let inner;
			let aliases = Aliases {
				alias: input.parse()?,
				eq: input.parse()?,
				bracket: bracketed!(inner in input),
				aliases: inner.parse_terminated(Ident::parse)?,
			};
			Ok(aliases)
		} else {
			Err(lookahead.error())
		}
	}
}

impl Parse for PropertyAttribute {
	fn parse(input: ParseStream) -> Result<PropertyAttribute> {
		use PropertyAttribute::*;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::convert) {
			Ok(Convert {
				convert: input.parse()?,
				eq: input.parse()?,
				conversion: input.parse()?,
			})
		} else if lookahead.peek(keywords::alias) {
			Ok(Alias(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

impl Parse for MethodAttribute {
	fn parse(input: ParseStream) -> Result<MethodAttribute> {
		use MethodAttribute::*;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::constructor) {
			Ok(Constructor(input.parse()?))
		} else if lookahead.peek(keywords::get) {
			Ok(Getter(input.parse()?))
		} else if lookahead.peek(keywords::set) {
			Ok(Setter(input.parse()?))
		} else if lookahead.peek(keywords::internal) {
			Ok(Internal(input.parse()?))
		} else if lookahead.peek(keywords::alias) {
			Ok(Alias(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}
