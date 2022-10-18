/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Expr, LitStr, Result};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Bracket;

use crate::class::method::MethodKind;

mod keywords {
	custom_keyword!(name);
	custom_keyword!(alias);
	custom_keyword!(skip);

	custom_keyword!(no_constructor);
	custom_keyword!(from_jsval);
	custom_keyword!(to_jsval);
	custom_keyword!(into_jsval);

	custom_keyword!(convert);
	custom_keyword!(readonly);

	custom_keyword!(constructor);
	custom_keyword!(get);
	custom_keyword!(set);
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Name {
	pub(crate) name: keywords::name,
	pub(crate) eq: Token![=],
	pub(crate) literal: LitStr,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Aliases {
	pub(crate) alias: keywords::alias,
	pub(crate) eq: Token![=],
	pub(crate) bracket: Bracket,
	pub(crate) aliases: Punctuated<LitStr, Token![,]>,
}

// TODO: Add `inspectable` to provide `toString` and `toJSON`
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum ClassAttribute {
	Name(Name),
	NoConstructor(keywords::no_constructor),
	FromJSVal(keywords::from_jsval),
	ToJSVal(keywords::to_jsval),
	IntoJSVal(keywords::into_jsval),
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum PropertyAttribute {
	Name(Name),
	Alias(Aliases),
	Convert {
		convert: keywords::convert,
		eq: Token![=],
		conversion: Box<Expr>,
	},
	Readonly(keywords::readonly),
	Skip(keywords::skip),
}

#[derive(Debug)]
pub(crate) enum MethodAttribute {
	Name(Name),
	Alias(Aliases),
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
			MA::Skip(_) => Some(MethodKind::Internal),
			_ => None,
		}
	}
}

impl Parse for Name {
	fn parse(input: ParseStream) -> Result<Name> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(Name {
				name: input.parse()?,
				eq: input.parse()?,
				literal: input.parse()?,
			})
		} else {
			Err(lookahead.error())
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
				aliases: inner.parse_terminated(<LitStr as Parse>::parse)?,
			};
			Ok(aliases)
		} else {
			Err(lookahead.error())
		}
	}
}

impl Parse for ClassAttribute {
	fn parse(input: ParseStream) -> Result<ClassAttribute> {
		use ClassAttribute as CA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(CA::Name(input.parse()?))
		} else if lookahead.peek(keywords::no_constructor) {
			Ok(CA::NoConstructor(input.parse()?))
		} else if lookahead.peek(keywords::from_jsval) {
			Ok(CA::FromJSVal(input.parse()?))
		} else if lookahead.peek(keywords::to_jsval) {
			Ok(CA::ToJSVal(input.parse()?))
		} else if lookahead.peek(keywords::into_jsval) {
			Ok(CA::IntoJSVal(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

impl Parse for PropertyAttribute {
	fn parse(input: ParseStream) -> Result<PropertyAttribute> {
		use PropertyAttribute as PA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(PA::Name(input.parse()?))
		} else if lookahead.peek(keywords::alias) {
			Ok(PA::Alias(input.parse()?))
		} else if lookahead.peek(keywords::convert) {
			Ok(PA::Convert {
				convert: input.parse()?,
				eq: input.parse()?,
				conversion: input.parse()?,
			})
		} else if lookahead.peek(keywords::readonly) {
			Ok(PA::Readonly(input.parse()?))
		} else if lookahead.peek(keywords::skip) {
			Ok(PA::Skip(input.parse()?))
		} else {
			Err(lookahead.error())
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
