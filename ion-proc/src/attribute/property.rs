/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::parse::{Parse, ParseStream};
use syn::Result;

use crate::attribute::class::{AliasAttribute, NameAttribute};

mod keywords {
	custom_keyword!(name);
	custom_keyword!(alias);
	custom_keyword!(skip);
}

pub(crate) enum PropertyAttribute {
	Name(NameAttribute),
	Alias(AliasAttribute),
	Skip(keywords::skip),
}

impl Parse for PropertyAttribute {
	fn parse(input: ParseStream) -> Result<PropertyAttribute> {
		use PropertyAttribute as PA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(PA::Name(input.parse()?))
		} else if lookahead.peek(keywords::alias) {
			Ok(PA::Alias(input.parse()?))
		} else if lookahead.peek(keywords::skip) {
			Ok(PA::Skip(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}
