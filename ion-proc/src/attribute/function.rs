/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Expr, Result};
use syn::meta::ParseNestedMeta;

use crate::attribute::{ParseArgument, ParseAttribute};

#[derive(Default)]
pub(crate) struct ParameterAttribute {
	pub(crate) this: bool,
	pub(crate) varargs: bool,
	pub(crate) convert: Option<Box<Expr>>,
	pub(crate) strict: bool,
}

impl ParseAttribute for ParameterAttribute {
	fn parse(&mut self, meta: &ParseNestedMeta) -> Result<()> {
		self.this.parse_argument(meta, "this", "Parameter")?;
		self.varargs.parse_argument(meta, "varargs", "Parameter")?;
		self.convert.parse_argument(meta, "convert", "Parameter")?;
		self.strict.parse_argument(meta, "strict", "Parameter")?;

		if self.this && (self.varargs || self.convert.is_some() || self.strict) {
			return Err(
				meta.error("Parameter with `this` attribute cannot have `varargs`, `convert`, or `strict` attributes.")
			);
		}

		Ok(())
	}
}
