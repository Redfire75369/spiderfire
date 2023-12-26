/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{LitStr, Result};
use syn::meta::ParseNestedMeta;

use crate::attribute::name::{Name, NameArgument};
use crate::attribute::name::AliasArgument;
use crate::attribute::ParseAttribute;

#[derive(Default)]
pub struct PropertyAttribute {
	pub(crate) name: Option<Name>,
	pub(crate) alias: Vec<LitStr>,
	pub(crate) skip: bool,
	pub(crate) r#static: bool,
}

impl ParseAttribute for PropertyAttribute {
	fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
		if meta.path.is_ident("name") {
			let name: NameArgument = meta.input.parse()?;
			if self.name.is_some() {
				return Err(meta.error("Property cannot have multiple `name` attributes."));
			}
			self.name = Some(name.name);
		} else if meta.path.is_ident("alias") {
			let alias: AliasArgument = meta.input.parse()?;
			self.alias.extend(alias.aliases);
		} else if meta.path.is_ident("skip") {
			if self.skip {
				return Err(meta.error("Property cannot have multiple `skip` attributes."));
			}
			self.skip = true;
		} else if meta.path.is_ident("static") {
			if self.r#static {
				return Err(meta.error("Property cannot have multiple `static` attributes."));
			}
			self.r#static = true;
		}

		Ok(())
	}
}
