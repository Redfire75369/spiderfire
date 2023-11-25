/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use syn::{ImplItemFn, parse2, Result, Type};

use crate::attribute::class::Name;
use crate::class::accessor::{Accessor, flatten_accessors};
use crate::class::method::Method;
use crate::class::property::Property;

#[derive(Default)]
pub(super) struct PrototypeSpecs {
	pub(super) methods: (Vec<Method>, Vec<Method>),
	pub(super) properties: (Vec<Property>, Vec<Property>),
	pub(super) accessors: (HashMap<String, Accessor>, HashMap<String, Accessor>),
}

impl PrototypeSpecs {
	pub(super) fn to_spec_functions(&self, ion: &TokenStream, span: Span, ident: &Ident) -> Result<SpecFunctions> {
		Ok(SpecFunctions {
			methods: (
				methods_to_spec_function(ion, span, ident, &self.methods.0, false)?,
				methods_to_spec_function(ion, span, ident, &self.methods.1, true)?,
			),
			properties: (
				properties_to_spec_function(ion, span, ident, &self.properties.0, &self.accessors.0, false)?,
				properties_to_spec_function(ion, span, ident, &self.properties.1, &self.accessors.1, true)?,
			),
		})
	}

	pub(super) fn into_functions(self) -> Vec<Method> {
		let mut functions = Vec::with_capacity(
			self.methods.0.len() + self.methods.1.len() + self.accessors.0.len() + self.accessors.1.len(),
		);

		functions.extend(self.methods.0);
		functions.extend(self.methods.1);
		functions.extend(flatten_accessors(self.accessors.0));
		functions.extend(flatten_accessors(self.accessors.1));

		functions
	}
}

pub(super) struct SpecFunctions {
	methods: (ImplItemFn, ImplItemFn),
	properties: (ImplItemFn, ImplItemFn),
}

impl SpecFunctions {
	pub(super) fn into_array(self) -> [ImplItemFn; 4] {
		[self.methods.0, self.properties.0, self.methods.1, self.properties.1]
	}
}

fn methods_to_spec_function(
	ion: &TokenStream, span: Span, class: &Ident, methods: &[Method], r#static: bool,
) -> Result<ImplItemFn> {
	let ident = if r#static {
		parse_quote!(ION_STATIC_FUNCTIONS)
	} else {
		parse_quote!(ION_FUNCTIONS)
	};
	let function_ident = if r#static {
		parse_quote!(__ion_static_function_specs)
	} else {
		parse_quote!(__ion_function_specs)
	};

	let mut specs: Vec<_> = methods
		.iter()
		.flat_map(|method| {
			let ident = method.method.sig.ident.clone();
			let nargs = method.nargs as u16;
			(*method.names)
				.iter()
				.map(|name| match name {
					Name::String(literal) => {
						let mut name = literal.value();
						if name.is_case(Case::Snake) {
							name = name.to_case(Case::Camel)
						}
						quote!(#ion::function_spec!(#class::#ident, #name, #nargs, #ion::flags::PropertyFlags::CONSTANT_ENUMERATED))
					}
					Name::Symbol(symbol) => {
						quote!(#ion::function_spec_symbol!(#class::#ident, #symbol, #nargs, #ion::flags::PropertyFlags::CONSTANT))
					}
				})
				.collect::<Vec<_>>()
		})
		.collect();
	specs.push(parse_quote!(::mozjs::jsapi::JSFunctionSpec::ZERO));

	spec_function(
		span,
		&ident,
		&function_ident,
		&specs,
		parse_quote!(::mozjs::jsapi::JSFunctionSpec),
	)
}

fn properties_to_spec_function(
	ion: &TokenStream, span: Span, class: &Ident, properties: &[Property], accessors: &HashMap<String, Accessor>,
	r#static: bool,
) -> Result<ImplItemFn> {
	let ident: Ident = if r#static {
		parse_quote!(ION_STATIC_PROPERTIES)
	} else {
		parse_quote!(ION_PROPERTIES)
	};
	let function_ident: Ident = if r#static {
		parse_quote!(__ion_static_property_specs)
	} else {
		parse_quote!(__ion_property_specs)
	};

	let mut specs: Vec<_> = properties.iter().flat_map(|property| property.to_specs(ion, class)).collect();
	accessors.values().for_each(|accessor| specs.extend(accessor.to_specs(ion, class)));
	specs.push(parse_quote!(::mozjs::jsapi::JSPropertySpec::ZERO));

	spec_function(
		span,
		&ident,
		&function_ident,
		&specs,
		parse_quote!(::mozjs::jsapi::JSPropertySpec),
	)
}

fn spec_function(
	span: Span, ident: &Ident, function_ident: &Ident, specs: &[TokenStream], ty: Type,
) -> Result<ImplItemFn> {
	parse2(quote_spanned!(span => fn #function_ident() -> &'static [#ty] {
		static #ident: &[#ty] = &[
			#(#specs),*
		];
		#ident
	}))
}
