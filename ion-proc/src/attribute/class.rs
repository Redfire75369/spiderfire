/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Error, LitStr, Result};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

use crate::attribute::AttributeExt;
use crate::attribute::name::{AliasArgument, Name, NameArgument};
use crate::class::method::MethodKind;

mod keywords {
	custom_keyword!(name);
	custom_keyword!(alias);
	custom_keyword!(skip);

	custom_keyword!(class);

	custom_keyword!(convert);

	custom_keyword!(constructor);
	custom_keyword!(get);
	custom_keyword!(set);
}

#[allow(dead_code)]
pub(crate) struct ClassNameAttribute {
	_kw: keywords::name,
	_eq: Token![=],
	pub(crate) name: LitStr,
}

impl Parse for ClassNameAttribute {
	fn parse(input: ParseStream) -> Result<ClassNameAttribute> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(ClassNameAttribute {
				_kw: input.parse()?,
				_eq: input.parse()?,
				name: input.parse()?,
			})
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

pub(crate) enum MethodAttributeArgument {
	Name(NameArgument),
	Alias(AliasArgument),
	Constructor(keywords::constructor),
	Getter(keywords::get),
	Setter(keywords::set),
	Skip(keywords::skip),
}

impl Parse for MethodAttributeArgument {
	fn parse(input: ParseStream) -> Result<MethodAttributeArgument> {
		use MethodAttributeArgument as MA;

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

#[derive(Default)]
pub(crate) struct MethodAttribute {
	pub(crate) name: Option<Name>,
	pub(crate) alias: Vec<LitStr>,
	pub(crate) kind: Option<MethodKind>,
	pub(crate) skip: bool,
}

impl Parse for MethodAttribute {
	fn parse(input: ParseStream) -> Result<MethodAttribute> {
		use MethodAttributeArgument as MAA;
		let mut attribute = MethodAttribute::default();
		let span = input.span();

		let args = Punctuated::<MAA, Token![,]>::parse_terminated(input)?;
		for arg in args {
			match arg {
				MethodAttributeArgument::Name(name) => {
					if attribute.name.is_some() {
						return Err(Error::new(span, "Method cannot have multiple `name` attributes."));
					}
					attribute.name = Some(name.name);
				}
				MethodAttributeArgument::Alias(alias) => {
					attribute.alias.extend(alias.aliases);
				}
				MethodAttributeArgument::Constructor(_) => {
					if attribute.kind.is_some() {
						return Err(Error::new(
							span,
							"Method cannot have multiple `constructor`, `get`, or `set` attributes.",
						));
					}
					attribute.kind = Some(MethodKind::Constructor);
				}
				MethodAttributeArgument::Getter(_) => {
					if attribute.kind.is_some() {
						return Err(Error::new(
							span,
							"Method cannot have multiple `constructor`, `get`, or `set` attributes.",
						));
					}
					attribute.kind = Some(MethodKind::Getter);
				}
				MethodAttributeArgument::Setter(_) => {
					if attribute.kind.is_some() {
						return Err(Error::new(
							span,
							"Method cannot have multiple `constructor`, `get`, or `set` attributes.",
						));
					}
					attribute.kind = Some(MethodKind::Setter);
				}
				MethodAttributeArgument::Skip(_) => {
					if attribute.skip {
						return Err(Error::new(span, "Method cannot have multiple `skip` attributes."));
					}
					attribute.skip = true;
				}
			}
		}

		Ok(attribute)
	}
}

impl AttributeExt for MethodAttribute {}
