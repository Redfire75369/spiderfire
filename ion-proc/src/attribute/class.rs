/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{LitStr, Result};
use syn::meta::ParseNestedMeta;
use syn::parse::{Parse, ParseStream};

use crate::attribute::{ArgumentError, ParseArgument, ParseArgumentWith, ParseAttribute};
use crate::attribute::name::Name;
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
	fn parse(&mut self, meta: &ParseNestedMeta) -> Result<()> {
		const METHOD_KIND_ERROR: ArgumentError =
			ArgumentError::Full("Method cannot have multiple `constructor`, `get`, or `set` attributes.");

		self.name.parse_argument(meta, "name", "Method")?;
		self.alias.parse_argument(meta, "alias", None)?;
		self.kind
			.parse_argument_with(meta, MethodKind::Constructor, "constructor", METHOD_KIND_ERROR)?;
		self.kind.parse_argument_with(meta, MethodKind::Getter, "get", METHOD_KIND_ERROR)?;
		self.kind.parse_argument_with(meta, MethodKind::Setter, "set", METHOD_KIND_ERROR)?;
		self.skip.parse_argument(meta, "skip", "Method")?;

		Ok(())
	}
}
