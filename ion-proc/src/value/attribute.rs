/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Expr, ExprClosure, Lit, LitStr, Result};
use syn::parse::{Parse, ParseStream};

mod keywords {
	custom_keyword!(tag);
	custom_keyword!(untagged);

	custom_keyword!(inherit);
	custom_keyword!(skip);

	custom_keyword!(name);
	custom_keyword!(convert);
	custom_keyword!(strict);
	custom_keyword!(parser);
}

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) enum Tag {
	Untagged(keywords::untagged),
	External(keywords::tag),
	Internal { kw: keywords::tag, eq: Token![=], key: LitStr },
}

impl Default for Tag {
	fn default() -> Tag {
		Tag::Untagged(keywords::untagged::default())
	}
}

impl Parse for Tag {
	fn parse(input: ParseStream) -> Result<Tag> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::untagged) {
			Ok(Tag::Untagged(input.parse()?))
		} else if lookahead.peek(keywords::tag) {
			let kw = input.parse()?;
			let lookahead = input.lookahead1();
			if lookahead.peek(Token![=]) {
				Ok(Tag::Internal {
					kw,
					eq: input.parse()?,
					key: input.parse()?,
				})
			} else {
				Ok(Tag::External(kw))
			}
		} else {
			Err(lookahead.error())
		}
	}
}

pub(crate) enum DefaultValue {
	Literal(Lit),
	Closure(ExprClosure),
	Expr(Box<Expr>),
}

impl Parse for DefaultValue {
	fn parse(input: ParseStream) -> Result<DefaultValue> {
		let expr: Expr = input.parse()?;
		match expr {
			Expr::Lit(lit) => Ok(DefaultValue::Literal(lit.lit)),
			Expr::Closure(closure) => Ok(DefaultValue::Closure(closure)),
			expr => Ok(DefaultValue::Expr(Box::new(expr))),
		}
	}
}

pub(crate) enum DataAttribute {
	Tag(Tag),
	Inherit(keywords::inherit),
}

impl Parse for DataAttribute {
	fn parse(input: ParseStream) -> Result<DataAttribute> {
		use DataAttribute as DA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::untagged) || lookahead.peek(keywords::tag) {
			Ok(DA::Tag(input.parse()?))
		} else if lookahead.peek(keywords::inherit) {
			Ok(DA::Inherit(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

#[allow(dead_code)]
pub(crate) enum VariantAttribute {
	Tag(Tag),
	Inherit(keywords::inherit),
	Skip(keywords::skip),
}

impl Parse for VariantAttribute {
	fn parse(input: ParseStream) -> Result<VariantAttribute> {
		use VariantAttribute as VA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::untagged) || lookahead.peek(keywords::tag) {
			Ok(VA::Tag(input.parse()?))
		} else if lookahead.peek(keywords::inherit) {
			Ok(VA::Inherit(input.parse()?))
		} else if lookahead.peek(keywords::skip) {
			Ok(VA::Skip(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

#[allow(dead_code)]
pub(crate) enum FieldAttribute {
	Name {
		kw: keywords::name,
		eq: Token![=],
		name: LitStr,
	},
	Inherit(keywords::inherit),
	Skip(keywords::skip),
	Convert {
		kw: keywords::convert,
		eq: Token![=],
		expr: Box<Expr>,
	},
	Strict(keywords::strict),
	Default {
		kw: Token![default],
		eq: Option<Token![=]>,
		def: Option<DefaultValue>,
	},
	Parser {
		kw: keywords::parser,
		eq: Token![=],
		expr: Box<Expr>,
	},
}

impl Parse for FieldAttribute {
	fn parse(input: ParseStream) -> Result<FieldAttribute> {
		use FieldAttribute as FA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(FA::Name {
				kw: input.parse()?,
				eq: input.parse()?,
				name: input.parse()?,
			})
		} else if lookahead.peek(keywords::inherit) {
			Ok(FA::Inherit(input.parse()?))
		} else if lookahead.peek(keywords::skip) {
			Ok(FA::Skip(input.parse()?))
		} else if lookahead.peek(keywords::convert) {
			Ok(FA::Convert {
				kw: input.parse()?,
				eq: input.parse()?,
				expr: input.parse()?,
			})
		} else if lookahead.peek(keywords::strict) {
			Ok(FA::Strict(input.parse()?))
		} else if lookahead.peek(Token![default]) {
			let kw = input.parse()?;
			let eq: Option<_> = input.parse()?;
			let def = eq.map(|_| input.parse()).transpose()?;
			Ok(FA::Default { kw, eq, def })
		} else if lookahead.peek(keywords::parser) {
			Ok(FA::Parser {
				kw: input.parse()?,
				eq: input.parse()?,
				expr: input.parse()?,
			})
		} else {
			Err(lookahead.error())
		}
	}
}
