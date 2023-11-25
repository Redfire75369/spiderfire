/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Span;
use syn::{Error, ExprPath, LitStr, Result};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Bracket;

use crate::class::method::MethodKind;

mod keywords {
	custom_keyword!(name);
	custom_keyword!(alias);
	custom_keyword!(skip);

	custom_keyword!(class);

	custom_keyword!(convert);
	custom_keyword!(readonly);

	custom_keyword!(constructor);
	custom_keyword!(get);
	custom_keyword!(set);
}

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
}

impl Parse for Name {
	fn parse(input: ParseStream) -> Result<Name> {
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

#[allow(dead_code)]
pub(crate) struct NameAttribute {
	kw: keywords::name,
	eq: Token![=],
	pub(crate) name: Name,
}

impl Parse for NameAttribute {
	fn parse(input: ParseStream) -> Result<NameAttribute> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(NameAttribute {
				kw: input.parse()?,
				eq: input.parse()?,
				name: input.parse()?,
			})
		} else {
			Err(lookahead.error())
		}
	}
}

#[allow(dead_code)]
pub(crate) struct ClassNameAttribute {
	kw: keywords::name,
	eq: Token![=],
	pub(crate) name: LitStr,
}

impl Parse for ClassNameAttribute {
	fn parse(input: ParseStream) -> Result<ClassNameAttribute> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(ClassNameAttribute {
				kw: input.parse()?,
				eq: input.parse()?,
				name: input.parse()?,
			})
		} else {
			Err(lookahead.error())
		}
	}
}

#[allow(dead_code)]
pub(crate) struct AliasAttribute {
	kw: keywords::alias,
	eq: Token![=],
	bracket: Bracket,
	pub(crate) aliases: Punctuated<LitStr, Token![,]>,
}

impl Parse for AliasAttribute {
	fn parse(input: ParseStream) -> Result<AliasAttribute> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::alias) {
			let inner;
			let aliases = AliasAttribute {
				kw: input.parse()?,
				eq: input.parse()?,
				bracket: bracketed!(inner in input),
				aliases: inner.parse_terminated(<LitStr as Parse>::parse, Token![,])?,
			};
			Ok(aliases)
		} else {
			Err(lookahead.error())
		}
	}
}

// TODO: Add `inspectable` to provide `toString` and `toJSON`
#[allow(dead_code)]
pub(crate) enum ClassAttribute {
	Name(ClassNameAttribute),
	Class(keywords::class),
}

impl Parse for ClassAttribute {
	fn parse(input: ParseStream) -> Result<ClassAttribute> {
		use ClassAttribute as CA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(CA::Name(input.parse()?))
		} else if lookahead.peek(keywords::class) {
			Ok(CA::Class(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

pub(crate) enum MethodAttribute {
	Name(NameAttribute),
	Alias(AliasAttribute),
	Constructor(keywords::constructor),
	Getter(keywords::get),
	Setter(keywords::set),
	Skip(keywords::skip),
}

impl MethodAttribute {
	pub(crate) fn to_kind(&self) -> Option<MethodKind> {
		use MethodAttribute as MA;
		match self {
			MA::Constructor(_) => Some(MethodKind::Constructor),
			MA::Getter(_) => Some(MethodKind::Getter),
			MA::Setter(_) => Some(MethodKind::Setter),
			_ => None,
		}
	}
}

impl Parse for MethodAttribute {
	fn parse(input: ParseStream) -> Result<MethodAttribute> {
		use MethodAttribute as MA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(MA::Name(input.parse()?))
		} else if lookahead.peek(keywords::alias) {
			Ok(MA::Alias(input.parse()?))
		} else if lookahead.peek(keywords::constructor) {
			Ok(MA::Constructor(input.parse()?))
		} else if lookahead.peek(keywords::get) {
			Ok(MA::Getter(input.parse()?))
		} else if lookahead.peek(keywords::set) {
			Ok(MA::Setter(input.parse()?))
		} else if lookahead.peek(keywords::skip) {
			Ok(MA::Skip(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}
