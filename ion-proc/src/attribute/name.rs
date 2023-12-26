/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use syn::{Error, Expr, ExprLit, ExprPath, Lit, LitStr};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Bracket;

#[derive(Clone)]
pub(crate) enum Name {
	String(LitStr),
	Symbol(ExprPath),
}

impl Name {
	pub(crate) fn from_string<S: AsRef<str>>(string: S, span: Span) -> Name {
		Name::String(LitStr::new(string.as_ref(), span))
	}

	pub(crate) fn as_string(&self) -> String {
		match self {
			Name::String(literal) => literal.value(),
			Name::Symbol(path) => path.path.segments.last().map(|segment| format!("[{}]", segment.ident)).unwrap(),
		}
	}

	pub(crate) fn to_property_spec(&self, ion: &TokenStream, function: &mut Ident) -> (Box<Expr>, Box<Expr>) {
		match self {
			Name::String(literal) => {
				let mut name = literal.value();
				if name.is_case(Case::ScreamingSnake) {
					name = name.to_case(Case::Camel)
				}
				(
					Box::new(Expr::Lit(ExprLit {
						attrs: Vec::new(),
						lit: Lit::Str(LitStr::new(&name, literal.span())),
					})),
					parse_quote!(#ion::flags::PropertyFlags::CONSTANT_ENUMERATED),
				)
			}
			Name::Symbol(symbol) => {
				*function = format_ident!("{}_symbol", function);
				(
					Box::new(Expr::Path(symbol.clone())),
					parse_quote!(#ion::flags::PropertyFlags::CONSTANT),
				)
			}
		}
	}
}

impl Parse for Name {
	fn parse(input: ParseStream) -> syn::Result<Name> {
		if let Ok(literal) = input.parse::<LitStr>() {
			let string = literal.value();
			if !string.starts_with('[') && !string.ends_with(']') {
				Ok(Name::String(literal))
			} else {
				Err(Error::new(
					literal.span(),
					"Function name must not start with '[' or end with ']'",
				))
			}
		} else if let Ok(other) = input.parse() {
			Ok(Name::Symbol(other))
		} else {
			Err(Error::new(input.span(), "Function name is not a string or expression"))
		}
	}
}

pub(crate) struct NameArgument {
	_eq: Token![=],
	pub(crate) name: Name,
}

impl Parse for NameArgument {
	fn parse(input: ParseStream) -> syn::Result<NameArgument> {
		Ok(NameArgument {
			_eq: input.parse()?,
			name: input.parse()?,
		})
	}
}

pub(crate) struct AliasArgument {
	_eq: Token![=],
	_bracket: Bracket,
	pub(crate) aliases: Punctuated<LitStr, Token![,]>,
}

impl Parse for AliasArgument {
	fn parse(input: ParseStream) -> syn::Result<AliasArgument> {
		let inner;
		let aliases = AliasArgument {
			_eq: input.parse()?,
			_bracket: bracketed!(inner in input),
			aliases: inner.parse_terminated(<LitStr as Parse>::parse, Token![,])?,
		};
		Ok(aliases)
	}
}
