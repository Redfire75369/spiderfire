/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Expr, ExprClosure, Lit, LitStr, Result};
use syn::meta::ParseNestedMeta;
use syn::parse::{Parse, ParseStream};

use crate::attribute::function::ConvertArgument;
use crate::attribute::ParseAttribute;

#[derive(Clone, Default)]
pub(crate) enum Tag {
	#[default]
	Untagged,
	External,
	Internal(LitStr),
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

#[derive(Default)]
pub(crate) struct DataAttribute {
	pub(crate) tag: Option<Tag>,
	pub(crate) inherit: bool,
}

impl ParseAttribute for DataAttribute {
	fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
		const TAG_ERROR: &str = "Data cannot have multiple `untagged`, or `tag` attributes.";

		if meta.path.is_ident("untagged") {
			if self.tag.is_some() {
				return Err(meta.error(TAG_ERROR));
			}
			self.tag = Some(Tag::Untagged);
		} else if meta.path.is_ident("tag") {
			let eq: Option<Token![=]> = meta.input.parse()?;
			let tag: Option<LitStr> = eq.map(|_| meta.input.parse()).transpose()?;

			if self.tag.is_some() {
				return Err(meta.error(TAG_ERROR));
			}
			self.tag = Some(tag.map(Tag::Internal).unwrap_or(Tag::External));
		} else if meta.path.is_ident("inherit") {
			if self.inherit {
				return Err(meta.error("Variant cannot have multiple `inherit` attributes."));
			}
			self.inherit = true;
		}

		Ok(())
	}
}

#[derive(Default)]
pub(crate) struct VariantAttribute {
	pub(crate) tag: Option<Tag>,
	pub(crate) inherit: bool,
	pub(crate) skip: bool,
}

impl ParseAttribute for VariantAttribute {
	fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
		const TAG_ERROR: &str = "Variant cannot have multiple `untagged`, or `tag` attributes.";

		if meta.path.is_ident("untagged") {
			if self.tag.is_some() {
				return Err(meta.error(TAG_ERROR));
			}
			self.tag = Some(Tag::Untagged);
		} else if meta.path.is_ident("tag") {
			let eq: Option<Token![=]> = meta.input.parse()?;
			let tag: Option<LitStr> = eq.map(|_| meta.input.parse()).transpose()?;

			if self.tag.is_some() {
				return Err(meta.error(TAG_ERROR));
			}
			self.tag = Some(tag.map(Tag::Internal).unwrap_or(Tag::External));
		} else if meta.path.is_ident("inherit") {
			if self.inherit {
				return Err(meta.error("Variant cannot have multiple `inherit` attributes."));
			}
			self.inherit = true;
		} else if meta.path.is_ident("skip") {
			if self.skip {
				return Err(meta.error("Variant cannot have multiple `skip` attributes."));
			}
			self.skip = true;
		}

		Ok(())
	}
}

#[derive(Default)]
pub(crate) struct FieldAttribute {
	pub(crate) name: Option<LitStr>,
	pub(crate) inherit: bool,
	pub(crate) skip: bool,
	pub(crate) convert: Option<Box<Expr>>,
	pub(crate) strict: bool,
	pub(crate) default: Option<Option<DefaultValue>>,
	pub(crate) parser: Option<Box<Expr>>,
}

impl ParseAttribute for FieldAttribute {
	fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
		if meta.path.is_ident("name") {
			let _eq: Token![=] = meta.input.parse()?;
			let name: LitStr = meta.input.parse()?;
			if self.name.is_some() {
				return Err(meta.error("Field cannot have multiple `name` attributes."));
			}
			self.name = Some(name);
		} else if meta.path.is_ident("inherit") {
			if self.inherit {
				return Err(meta.error("Field cannot have multiple `inherit` attributes."));
			}
			self.inherit = true;
		} else if meta.path.is_ident("skip") {
			if self.skip {
				return Err(meta.error("Field cannot have multiple `skip` attributes."));
			}
			self.skip = true;
		} else if meta.path.is_ident("convert") {
			let convert: ConvertArgument = meta.input.parse()?;
			if self.convert.is_some() {
				return Err(meta.error("Field cannot have multiple `convert` attributes."));
			}
			self.convert = Some(convert.conversion);
		} else if meta.path.is_ident("strict") {
			if self.strict {
				return Err(meta.error("Field cannot have multiple `strict` attributes."));
			}
			self.strict = true;
		} else if meta.path.is_ident("default") {
			let eq: Option<Token![=]> = meta.input.parse()?;
			let def = eq.map(|_| meta.input.parse()).transpose()?;

			if self.default.is_some() {
				return Err(meta.error("Field cannot have multiple `default` attributes."));
			}
			self.default = Some(def);
		} else if meta.path.is_ident("parser") {
			let _eq: Token![=] = meta.input.parse()?;
			let expr: Box<Expr> = meta.input.parse()?;
			if self.parser.is_some() {
				return Err(meta.error("Field cannot have multiple `parser` attributes."));
			}
			self.parser = Some(expr);
		}

		Ok(())
	}
}
