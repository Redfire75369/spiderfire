/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Expr, Result};
use syn::meta::ParseNestedMeta;
use syn::parse::{Parse, ParseStream};

use crate::attribute::ParseAttribute;

pub(crate) struct ConvertArgument {
	_eq: Token![=],
	pub(crate) conversion: Box<Expr>,
}

impl Parse for ConvertArgument {
	fn parse(input: ParseStream) -> Result<ConvertArgument> {
		Ok(ConvertArgument {
			_eq: input.parse()?,
			conversion: input.parse()?,
		})
	}
}

#[derive(Default)]
pub(crate) struct ParameterAttribute {
	pub(crate) this: bool,
	pub(crate) varargs: bool,
	pub(crate) convert: Option<Box<Expr>>,
	pub(crate) strict: bool,
}

impl ParseAttribute for ParameterAttribute {
	fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
		if meta.path.is_ident("this") {
			if self.this {
				return Err(meta.error("Parameter cannot have multiple `this` attributes."));
			}
			self.this = true;
		} else if meta.path.is_ident("varargs") {
			if self.varargs {
				return Err(meta.error("Parameter cannot have multiple `varargs` attributes."));
			}
			self.varargs = true;
		} else if meta.path.is_ident("convert") {
			let convert: ConvertArgument = meta.input.parse()?;
			if self.convert.is_some() {
				return Err(meta.error("Parameter cannot have multiple `convert` attributes."));
			}
			self.convert = Some(convert.conversion);
		} else if meta.path.is_ident("strict") {
			if self.strict {
				return Err(meta.error("Parameter cannot have multiple `strict` attributes."));
			}
			self.strict = true;
		}

		Ok(())
	}
}
