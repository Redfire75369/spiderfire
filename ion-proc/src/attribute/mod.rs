/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use syn::{Attribute, Result};
use syn::meta::ParseNestedMeta;

pub(crate) mod class;
pub(crate) mod function;
pub(crate) mod krate;
pub(crate) mod name;
pub(crate) mod property;
pub(crate) mod trace;
pub(crate) mod value;

pub(crate) trait ParseAttribute: Default {
	fn parse(&mut self, meta: ParseNestedMeta) -> Result<()>;

	fn from_attributes<I: ?Sized>(path: &I, attrs: &[Attribute]) -> Result<Self>
	where
		Ident: PartialEq<I>,
	{
		let mut attribute = Self::default();
		for attr in attrs {
			if attr.path().is_ident(path) {
				attr.parse_nested_meta(|meta| attribute.parse(meta))?;
			}
		}
		Ok(attribute)
	}

	fn from_attributes_mut<I: ?Sized>(path: &I, attrs: &mut Vec<Attribute>) -> Result<Self>
	where
		Ident: PartialEq<I>,
	{
		let mut indices = Vec::new();
		let mut attribute = Self::default();
		for (i, attr) in attrs.iter().enumerate() {
			if attr.path().is_ident(path) {
				attr.parse_nested_meta(|meta| attribute.parse(meta))?;
				indices.push(i);
				break;
			}
		}
		while let Some(index) = indices.pop() {
			attrs.remove(index);
		}
		Ok(attribute)
	}
}
