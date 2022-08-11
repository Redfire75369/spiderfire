/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use syn::{ImplItemConst, LitStr, Type};

use crate::utils::type_ends_with;

#[derive(Clone, Debug)]
pub(crate) enum Property {
	Int32(Ident),
	Double(Ident),
	String(Ident),
}

impl Property {
	pub(crate) fn from_const(con: &ImplItemConst) -> Option<Property> {
		match &con.ty {
			Type::Path(ty) => {
				if type_ends_with(&ty, "i32") {
					Some(Property::Int32(con.ident.clone()))
				} else if type_ends_with(&ty, "f64") {
					Some(Property::Double(con.ident.clone()))
				} else {
					None
				}
			}
			Type::Reference(re) => {
				if let Type::Path(ty) = &*re.elem {
					if type_ends_with(&ty, "str") {
						return Some(Property::String(con.ident.clone()));
					}
				}
				None
			}
			_ => None,
		}
	}

	pub(crate) fn to_spec(&self, class: Ident) -> TokenStream {
		let krate = quote!(::ion);
		match self {
			Property::Int32(ident) => {
				let name = LitStr::new(&(format!("{}\0", ident)), ident.span());
				quote!(#krate::spec::create_property_spec_int(#name, #class::#ident, #krate::flags::PropertyFlags::CONSTANT_ENUMERATED))
			}
			Property::Double(ident) => {
				let name = LitStr::new(&(format!("{}\0", ident)), ident.span());
				quote!(#krate::spec::create_property_spec_double(#name, #class::#ident, #krate::flags::PropertyFlags::CONSTANT_ENUMERATED))
			}
			Property::String(ident) => {
				let name = LitStr::new(&(format!("{}\0", ident)), ident.span());
				// TODO: Null-Terminate Constant
				quote!(#krate::spec::create_property_spec_string(#name, #class::#ident, #krate::flags::PropertyFlags::CONSTANT_ENUMERATED))
			}
		}
	}
}
