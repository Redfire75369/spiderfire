/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Error, Expr, ExprClosure, Lit, LitStr, Result};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

use crate::attribute::AttributeExt;
use crate::attribute::function::ConvertArgument;

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

#[derive(Clone)]
pub(crate) enum Tag {
	Untagged,
	External(keywords::tag),
	Internal(keywords::tag, LitStr),
}

impl Default for Tag {
	fn default() -> Tag {
		Tag::Untagged
	}
}

enum TagArgument {
	Untagged(keywords::untagged),
	External(keywords::tag),
	Internal { kw: keywords::tag, _eq: Token![=], key: LitStr },
}

impl TagArgument {
	fn into_tag(self) -> Tag {
		match self {
			TagArgument::Untagged(_) => Tag::Untagged,
			TagArgument::External(kw) => Tag::External(kw),
			TagArgument::Internal { kw, key, .. } => Tag::Internal(kw, key),
		}
	}
}

impl Parse for TagArgument {
	fn parse(input: ParseStream) -> Result<TagArgument> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::untagged) {
			Ok(TagArgument::Untagged(input.parse()?))
		} else if lookahead.peek(keywords::tag) {
			let kw = input.parse()?;
			let lookahead = input.lookahead1();
			if lookahead.peek(Token![=]) {
				Ok(TagArgument::Internal {
					kw,
					_eq: input.parse()?,
					key: input.parse()?,
				})
			} else {
				Ok(TagArgument::External(kw))
			}
		} else {
			Err(lookahead.error())
		}
	}
}

pub(crate) enum DefaultValueArgument {
	Literal(Lit),
	Closure(ExprClosure),
	Expr(Box<Expr>),
}

impl Parse for DefaultValueArgument {
	fn parse(input: ParseStream) -> Result<DefaultValueArgument> {
		let expr: Expr = input.parse()?;
		match expr {
			Expr::Lit(lit) => Ok(DefaultValueArgument::Literal(lit.lit)),
			Expr::Closure(closure) => Ok(DefaultValueArgument::Closure(closure)),
			expr => Ok(DefaultValueArgument::Expr(Box::new(expr))),
		}
	}
}

enum DataAttributeArgument {
	Tag(TagArgument),
	Inherit(keywords::inherit),
}

impl Parse for DataAttributeArgument {
	fn parse(input: ParseStream) -> Result<DataAttributeArgument> {
		use DataAttributeArgument as DAA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::untagged) || lookahead.peek(keywords::tag) {
			Ok(DAA::Tag(input.parse()?))
		} else if lookahead.peek(keywords::inherit) {
			Ok(DAA::Inherit(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

#[derive(Default)]
pub(crate) struct DataAttribute {
	pub(crate) tag: Option<Tag>,
	pub(crate) inherit: bool,
}

impl Parse for DataAttribute {
	fn parse(input: ParseStream) -> Result<DataAttribute> {
		use DataAttributeArgument as DAA;
		let mut attribute = DataAttribute::default();
		let span = input.span();

		let args = Punctuated::<DAA, Token![,]>::parse_terminated(input)?;
		for arg in args {
			match arg {
				DataAttributeArgument::Tag(tag) => {
					if attribute.tag.is_some() {
						return Err(Error::new(span, "Data cannot have multiple `tag` attributes."));
					}
					attribute.tag = Some(tag.into_tag());
				}
				DataAttributeArgument::Inherit(_) => {
					if attribute.inherit {
						return Err(Error::new(span, "Data cannot have multiple `inherit` attributes."));
					}
					attribute.inherit = true;
				}
			}
		}

		Ok(attribute)
	}
}

impl AttributeExt for DataAttribute {}

enum VariantAttributeArgument {
	Tag(TagArgument),
	Inherit(keywords::inherit),
	Skip(keywords::skip),
}

impl Parse for VariantAttributeArgument {
	fn parse(input: ParseStream) -> Result<VariantAttributeArgument> {
		use VariantAttributeArgument as VAA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::untagged) || lookahead.peek(keywords::tag) {
			Ok(VAA::Tag(input.parse()?))
		} else if lookahead.peek(keywords::inherit) {
			Ok(VAA::Inherit(input.parse()?))
		} else if lookahead.peek(keywords::skip) {
			Ok(VAA::Skip(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

#[derive(Default)]
pub(crate) struct VariantAttribute {
	pub(crate) tag: Option<Tag>,
	pub(crate) inherit: bool,
	pub(crate) skip: bool,
}

impl Parse for VariantAttribute {
	fn parse(input: ParseStream) -> Result<VariantAttribute> {
		use VariantAttributeArgument as VAA;
		let mut attribute = VariantAttribute::default();
		let span = input.span();

		let args = Punctuated::<VAA, Token![,]>::parse_terminated(input)?;
		for arg in args {
			match arg {
				VariantAttributeArgument::Tag(tag) => {
					if attribute.tag.is_some() {
						return Err(Error::new(span, "Variant cannot have multiple `tag` attributes."));
					}
					attribute.tag = Some(tag.into_tag());
				}
				VariantAttributeArgument::Inherit(_) => {
					if attribute.inherit {
						return Err(Error::new(span, "Variant cannot have multiple `inherit` attributes."));
					}
					attribute.inherit = true;
				}
				VariantAttributeArgument::Skip(_) => {
					if attribute.skip {
						return Err(Error::new(span, "Variant cannot have multiple `skip` attributes."));
					}
					attribute.skip = true;
				}
			}
		}

		Ok(attribute)
	}
}

impl AttributeExt for VariantAttribute {}

pub(crate) enum FieldAttributeArgument {
	Name {
		_kw: keywords::name,
		_eq: Token![=],
		name: LitStr,
	},
	Inherit(keywords::inherit),
	Skip(keywords::skip),
	Convert(ConvertArgument),
	Strict(keywords::strict),
	Default {
		_kw: Token![default],
		_eq: Option<Token![=]>,
		def: Option<DefaultValueArgument>,
	},
	Parser {
		_kw: keywords::parser,
		_eq: Token![=],
		expr: Box<Expr>,
	},
}

impl Parse for FieldAttributeArgument {
	fn parse(input: ParseStream) -> Result<FieldAttributeArgument> {
		use FieldAttributeArgument as FAA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(FAA::Name {
				_kw: input.parse()?,
				_eq: input.parse()?,
				name: input.parse()?,
			})
		} else if lookahead.peek(keywords::inherit) {
			Ok(FAA::Inherit(input.parse()?))
		} else if lookahead.peek(keywords::skip) {
			Ok(FAA::Skip(input.parse()?))
		} else if lookahead.peek(keywords::convert) {
			Ok(FAA::Convert(input.parse()?))
		} else if lookahead.peek(keywords::strict) {
			Ok(FAA::Strict(input.parse()?))
		} else if lookahead.peek(Token![default]) {
			let kw = input.parse()?;
			let eq: Option<_> = input.parse()?;
			let def = eq.map(|_| input.parse()).transpose()?;
			Ok(FAA::Default { _kw: kw, _eq: eq, def })
		} else if lookahead.peek(keywords::parser) {
			Ok(FAA::Parser {
				_kw: input.parse()?,
				_eq: input.parse()?,
				expr: input.parse()?,
			})
		} else {
			Err(lookahead.error())
		}
	}
}

#[derive(Default)]
pub(crate) struct FieldAttribute {
	pub(crate) name: Option<LitStr>,
	pub(crate) inherit: bool,
	pub(crate) skip: bool,
	pub(crate) convert: Option<Box<Expr>>,
	pub(crate) strict: bool,
	pub(crate) default: Option<Option<DefaultValueArgument>>,
	pub(crate) parser: Option<Box<Expr>>,
}

impl Parse for FieldAttribute {
	fn parse(input: ParseStream) -> Result<FieldAttribute> {
		use FieldAttributeArgument as FAA;
		let mut attribute = FieldAttribute::default();
		let span = input.span();

		let args = Punctuated::<FAA, Token![,]>::parse_terminated(input)?;
		for arg in args {
			match arg {
				FieldAttributeArgument::Name { name, .. } => {
					if attribute.name.is_some() {
						return Err(Error::new(span, "Field cannot have multiple `name` attributes."));
					}
					attribute.name = Some(name);
				}
				FieldAttributeArgument::Inherit(_) => {
					if attribute.inherit {
						return Err(Error::new(span, "Field cannot have multiple `inherit` attributes."));
					}
					attribute.inherit = true;
				}
				FieldAttributeArgument::Skip(_) => {
					if attribute.skip {
						return Err(Error::new(span, "Field cannot have multiple `skip` attributes."));
					}
					attribute.skip = true;
				}
				FieldAttributeArgument::Convert(ConvertArgument { conversion, .. }) => {
					if attribute.convert.is_some() {
						return Err(Error::new(span, "Field cannot have multiple `convert` attributes."));
					}
					attribute.convert = Some(conversion);
				}
				FieldAttributeArgument::Strict(_) => {
					if attribute.strict {
						return Err(Error::new(span, "Field cannot have multiple `strict` attributes."));
					}
					attribute.strict = true;
				}
				FieldAttributeArgument::Default { def, .. } => {
					if attribute.default.is_some() {
						return Err(Error::new(span, "Field cannot have multiple `default` attributes."));
					}
					attribute.default = Some(def);
				}
				FieldAttributeArgument::Parser { expr, .. } => {
					if attribute.parser.is_some() {
						return Err(Error::new(span, "Field cannot have multiple `parser` attributes."));
					}
					attribute.parser = Some(expr);
				}
			}
		}

		Ok(attribute)
	}
}

impl AttributeExt for FieldAttribute {}
