/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Error, LitStr, Result};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Static;

use crate::attribute::AttributeExt;
use crate::attribute::name::{Name, NameArgument};
use crate::attribute::name::AliasArgument;

mod keywords {
	custom_keyword!(name);
	custom_keyword!(alias);
	custom_keyword!(skip);
}

enum PropertyAttributeArgument {
	Name(NameArgument),
	Alias(AliasArgument),
	Skip(keywords::skip),
	Static(Static),
}

impl Parse for PropertyAttributeArgument {
	fn parse(input: ParseStream) -> Result<PropertyAttributeArgument> {
		use PropertyAttributeArgument as PAA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(PAA::Name(input.parse()?))
		} else if lookahead.peek(keywords::alias) {
			Ok(PAA::Alias(input.parse()?))
		} else if lookahead.peek(keywords::skip) {
			Ok(PAA::Skip(input.parse()?))
		} else if lookahead.peek(Static) {
			Ok(PAA::Static(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

#[derive(Default)]
pub struct PropertyAttribute {
	pub(crate) name: Option<Name>,
	pub(crate) alias: Vec<LitStr>,
	pub(crate) skip: bool,
	pub(crate) r#static: bool,
}

impl Parse for PropertyAttribute {
	fn parse(input: ParseStream) -> Result<PropertyAttribute> {
		use PropertyAttributeArgument as PAA;
		let mut attribute = PropertyAttribute {
			name: None,
			alias: Vec::new(),
			skip: false,
			r#static: false,
		};
		let span = input.span();

		let args = Punctuated::<PAA, Token![,]>::parse_terminated(input)?;
		for arg in args {
			match arg {
				PAA::Name(NameArgument { name, .. }) => {
					if attribute.name.is_some() {
						return Err(Error::new(span, "Property cannot have multiple `name` attributes."));
					}
					attribute.name = Some(name);
				}
				PAA::Alias(AliasArgument { aliases, .. }) => {
					attribute.alias.extend(aliases);
				}
				PAA::Skip(_) => {
					if attribute.skip {
						return Err(Error::new(span, "Property cannot have multiple `skip` attributes."));
					}
					attribute.skip = true;
				}
				PAA::Static(_) => {
					if attribute.r#static {
						return Err(Error::new(span, "Property cannot have multiple `static` attributes."));
					}
					attribute.r#static = true;
				}
			}
		}

		Ok(attribute)
	}
}

impl AttributeExt for PropertyAttribute {}
