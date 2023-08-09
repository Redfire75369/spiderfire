/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{ImplItemConst, LitStr, Result, Type};
use syn::punctuated::Punctuated;

use crate::attribute::class::Name;
use crate::attribute::property::PropertyAttribute;
use crate::utils::type_ends_with;

#[derive(Clone, Debug)]
pub(crate) enum PropertyType {
	Int32,
	Double,
	String,
}

#[derive(Clone, Debug)]
pub(crate) struct Property {
	ty: PropertyType,
	ident: Ident,
	names: Vec<Name>,
}

impl Property {
	pub(crate) fn from_const(mut con: ImplItemConst) -> Result<Option<(ImplItemConst, Property)>> {
		let mut name = None;
		let mut names = Vec::new();
		let mut skip = false;

		for attr in &con.attrs {
			if attr.path().is_ident("ion") {
				let args: Punctuated<PropertyAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

				for arg in args {
					match arg {
						PropertyAttribute::Name(name_) => name = Some(name_.name),
						PropertyAttribute::Alias(alias) => {
							for alias in alias.aliases {
								names.push(Name::String(alias));
							}
						}
						PropertyAttribute::Skip(_) => skip = true,
					}
				}
			}
		}

		con.attrs.clear();
		if skip {
			return Ok(None);
		}

		let ident = con.ident.clone();

		match name {
			Some(name) => names.insert(0, name),
			None => names.insert(0, Name::from_string(&ident.to_string(), ident.span())),
		}

		match &con.ty {
			Type::Path(ty) => {
				if type_ends_with(ty, "i32") {
					Ok(Some((con, Property { ty: PropertyType::Int32, ident, names })))
				} else if type_ends_with(ty, "f64") {
					Ok(Some((con, Property { ty: PropertyType::Double, ident, names })))
				} else {
					Ok(None)
				}
			}
			Type::Reference(re) => {
				if let Type::Path(ty) = &*re.elem {
					if type_ends_with(ty, "str") {
						return Ok(Some((con, Property { ty: PropertyType::String, ident, names })));
					}
				}
				Ok(None)
			}
			_ => Ok(None),
		}
	}

	pub(crate) fn to_specs(&self, class: &Ident) -> Vec<TokenStream> {
		let krate = quote!(::ion);
		let ident = &self.ident;

		self.names
			.iter()
			.map(|name| {
				let mut function_ident = Ident::new("create_property_spec", Span::call_site());
				let key;
				let flags;

				match name {
					Name::String(literal) => {
						let mut name = literal.value();
						if name.is_case(Case::ScreamingSnake) {
							name = name.to_case(Case::Camel)
						}
						key = LitStr::new(&name, literal.span()).into_token_stream();
						flags = quote!(#krate::flags::PropertyFlags::CONSTANT_ENUMERATED);
					}
					Name::Symbol(symbol) => {
						key = symbol.to_token_stream();
						function_ident = format_ident!("{}_symbol", function_ident);
						flags = quote!(#krate::flags::PropertyFlags::CONSTANT);
					}
				}

				match self.ty {
					PropertyType::Int32 => {
						function_ident = format_ident!("{}_int", function_ident);
					}
					PropertyType::Double => {
						function_ident = format_ident!("{}_double", function_ident);
					}
					PropertyType::String => {
						function_ident = format_ident!("{}_string", function_ident);
					}
				}

				quote!(#krate::spec::#function_ident(#key, #class::#ident, #flags))
			})
			.collect()
	}
}
