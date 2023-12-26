/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{LitStr, Result};
use syn::meta::ParseNestedMeta;
use syn::parse::{Parse, ParseStream};

use crate::attribute::name::{AliasArgument, Name, NameArgument};
use crate::attribute::ParseAttribute;
use crate::class::method::MethodKind;

mod keywords {
	custom_keyword!(name);
	custom_keyword!(class);
}

#[allow(dead_code)]
pub(crate) struct ClassNameAttribute {
	_kw: keywords::name,
	_eq: Token![=],
	pub(crate) name: LitStr,
}

impl Parse for ClassNameAttribute {
	fn parse(input: ParseStream) -> Result<ClassNameAttribute> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(ClassNameAttribute {
				_kw: input.parse()?,
				_eq: input.parse()?,
				name: input.parse()?,
			})
		} else {
			Err(lookahead.error())
		}
	}
}

// TODO: Add `inspectable` to provide `toString` and `toJSON`
#[allow(dead_code)]
pub(crate) enum ClassAttribute {
	Name(ClassNameAttribute),
	Class(keywords::class),
}

impl Parse for ClassAttribute {
	fn parse(input: ParseStream) -> Result<ClassAttribute> {
		use ClassAttribute as CA;

		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::name) {
			Ok(CA::Name(input.parse()?))
		} else if lookahead.peek(keywords::class) {
			Ok(CA::Class(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

#[derive(Default)]
pub(crate) struct MethodAttribute {
	pub(crate) name: Option<Name>,
	pub(crate) alias: Vec<LitStr>,
	pub(crate) kind: Option<MethodKind>,
	pub(crate) skip: bool,
}

impl ParseAttribute for MethodAttribute {
	fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
		const METHOD_KIND_ERROR: &str = "Method cannot have multiple `constructor`, `get`, or `set` attributes.";

		if meta.path.is_ident("name") {
			let name: NameArgument = meta.input.parse()?;
			if self.name.is_some() {
				return Err(meta.error("Method cannot have multiple `name` attributes."));
			}
			self.name = Some(name.name);
		} else if meta.path.is_ident("alias") {
			let alias: AliasArgument = meta.input.parse()?;
			self.alias.extend(alias.aliases);
		} else if meta.path.is_ident("constructor") {
			if self.kind.is_some() {
				return Err(meta.error(METHOD_KIND_ERROR));
			}
			self.kind = Some(MethodKind::Constructor);
		} else if meta.path.is_ident("get") {
			if self.kind.is_some() {
				return Err(meta.error(METHOD_KIND_ERROR));
			}
			self.kind = Some(MethodKind::Getter);
		} else if meta.path.is_ident("set") {
			if self.kind.is_some() {
				return Err(meta.error(METHOD_KIND_ERROR));
			}
			self.kind = Some(MethodKind::Setter);
		} else if meta.path.is_ident("skip") {
			if self.skip {
				return Err(meta.error("Method cannot have multiple `skip` attributes."));
			}
			self.skip = true;
		}

		Ok(())
	}
}
