/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Expr, Lit, Result};
use syn::parse::{Parse, ParseStream};

mod keywords {
	custom_keyword!(inherit);
	custom_keyword!(optional);

	custom_keyword!(convert);
	custom_keyword!(parser);
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum FromJSValAttribute {
	Inherit(keywords::inherit),
	Optional(keywords::optional),
	Convert {
		kw: keywords::convert,
		eq: Token!(=),
		expr: Expr,
	},
	Default {
		kw: Token!(default),
		eq: Option<Token!(=)>,
		def: Option<DefaultValue>,
	},
	Parser {
		kw: keywords::parser,
		eq: Token!(=),
		expr: Expr,
	},
}

#[derive(Debug)]
pub(crate) enum DefaultValue {
	Literal(Lit),
	Expr(Expr),
}

impl Parse for FromJSValAttribute {
	fn parse(input: ParseStream) -> Result<FromJSValAttribute> {
		use FromJSValAttribute::*;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::inherit) {
			Ok(Inherit(input.parse()?))
		} else if lookahead.peek(keywords::optional) {
			Ok(Optional(input.parse()?))
		} else if lookahead.peek(keywords::convert) {
			Ok(Convert {
				kw: input.parse()?,
				eq: input.parse()?,
				expr: input.parse()?,
			})
		} else if lookahead.peek(Token![default]) {
			let kw = input.parse()?;
			let eq: Option<_> = input.parse()?;
			let def = eq.map(|_| input.parse()).transpose()?;
			Ok(Default { kw, eq, def })
		} else if lookahead.peek(keywords::parser) {
			Ok(Parser {
				kw: input.parse()?,
				eq: input.parse()?,
				expr: input.parse()?,
			})
		} else {
			Err(lookahead.error())
		}
	}
}

impl Parse for DefaultValue {
	fn parse(input: ParseStream) -> Result<DefaultValue> {
		let expr: Expr = input.parse()?;
		match expr {
			Expr::Lit(lit) => Ok(DefaultValue::Literal(lit.lit)),
			expr => Ok(DefaultValue::Expr(expr)),
		}
	}
}
