/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use syn::{Attribute, Result};
use syn::parse::Parse;

pub(crate) mod class;
pub(crate) mod function;
pub(crate) mod krate;
pub(crate) mod property;
pub(crate) mod trace;

pub trait AttributeExt: Parse {
	fn from_attributes<I: ?Sized>(path: &I, attrs: &[Attribute]) -> Result<Option<Self>>
	where
		Ident: PartialEq<I>,
	{
		for attr in attrs {
			if attr.path().is_ident(path) {
				return Ok(Some(attr.parse_args()?));
			}
		}
		Ok(None)
	}

	fn from_attributes_mut<I: ?Sized>(path: &I, attrs: &mut Vec<Attribute>) -> Result<Option<Self>>
	where
		Ident: PartialEq<I>,
	{
		let mut attribute = None;
		for (i, attr) in attrs.iter().enumerate() {
			if attr.path().is_ident(path) {
				attribute = Some((i, attr.parse_args()?));
				break;
			}
		}
		if let Some((index, _)) = &attribute {
			attrs.remove(*index);
		}
		Ok(attribute.map(|a| a.1))
	}
}
