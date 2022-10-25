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
#[derive(Debug)]
pub(crate) enum ParameterAttribute {
	This(keywords::this),
	VarArgs(keywords::varargs),
	Convert {
		convert: keywords::convert,
		eq: Token![=],
		conversion: Box<Expr>,
	},
	Strict(keywords::strict),
}

impl Parse for ParameterAttribute {
	fn parse(input: ParseStream) -> Result<ParameterAttribute> {
		use ParameterAttribute::*;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::this) {
			Ok(This(input.parse()?))
		} else if lookahead.peek(keywords::varargs) {
			Ok(VarArgs(input.parse()?))
		} else if lookahead.peek(keywords::convert) {
			Ok(Convert {
				convert: input.parse()?,
				eq: input.parse()?,
				conversion: input.parse()?,
			})
		} else if lookahead.peek(keywords::strict) {
			Ok(input.parse()?)
		} else {
			Err(lookahead.error())
		}
	}
}
