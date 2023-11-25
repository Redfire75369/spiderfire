/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Expr, Result};
use syn::parse::{Parse, ParseStream};

mod keywords {
	custom_keyword!(this);
	custom_keyword!(varargs);
	custom_keyword!(convert);
	custom_keyword!(strict);
}

#[allow(dead_code)]
pub(crate) struct ConvertAttribute {
	kw: keywords::convert,
	eq: Token![=],
	pub(crate) conversion: Box<Expr>,
}

impl Parse for ConvertAttribute {
	fn parse(input: ParseStream) -> Result<ConvertAttribute> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::convert) {
			Ok(ConvertAttribute {
				kw: input.parse()?,
				eq: input.parse()?,
				conversion: input.parse()?,
			})
		} else {
			Err(lookahead.error())
		}
	}
}

pub(crate) enum ParameterAttribute {
	This(keywords::this),
	VarArgs(keywords::varargs),
	Convert(ConvertAttribute),
	Strict(keywords::strict),
}

impl Parse for ParameterAttribute {
	fn parse(input: ParseStream) -> Result<ParameterAttribute> {
		use ParameterAttribute as PA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::this) {
			Ok(PA::This(input.parse()?))
		} else if lookahead.peek(keywords::varargs) {
			Ok(PA::VarArgs(input.parse()?))
		} else if lookahead.peek(keywords::convert) {
			Ok(PA::Convert(input.parse()?))
		} else if lookahead.peek(keywords::strict) {
			Ok(PA::Strict(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}
