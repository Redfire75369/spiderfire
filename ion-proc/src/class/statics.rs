/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::ffi::CString;

use convert_case::{Case, Casing};
use proc_macro2::Ident;
use syn::{ItemImpl, ItemStatic, LitStr, parse2};

use crate::class::accessor::Accessor;
use crate::class::method::Method;
use crate::class::property::Property;

pub(crate) fn class_spec(literal: &LitStr) -> ItemStatic {
	let name = String::from_utf8(CString::new(literal.value()).unwrap().into_bytes_with_nul()).unwrap();
	let name = LitStr::new(&name, literal.span());

	parse_quote!(
		static CLASS: ::mozjs::jsapi::JSClass = ::mozjs::jsapi::JSClass {
			name: #name.as_ptr() as *const ::core::primitive::i8,
			flags: ::mozjs::jsapi::JSCLASS_HAS_PRIVATE,
			cOps: ::std::ptr::null_mut(),
			spec: ::std::ptr::null_mut(),
			ext: ::std::ptr::null_mut(),
			oOps: ::std::ptr::null_mut(),
		};
	)
}

pub(crate) fn methods_to_specs(methods: &[Method], stat: bool) -> ItemStatic {
	let krate = quote!(::ion);
	let ident = if stat { quote!(STATIC_FUNCTIONS) } else { quote!(FUNCTIONS) };
	let mut specs: Vec<_> = methods
		.into_iter()
		.flat_map(|method| {
			let ident = method.method.sig.ident.clone();
			let nargs = method.nargs as u16;
			(*method.names)
				.into_iter()
				.map(|name| {
					let mut string = name.value();
					if string.is_case(Case::Snake) {
						string = string.to_case(Case::Camel);
					}
					let name = LitStr::new(&string, name.span());

					quote!(#krate::function_spec!(#ident, #name, #nargs, #krate::flags::PropertyFlags::CONSTANT))
				})
				.collect::<Vec<_>>()
		})
		.collect();
	specs.push(quote!(::mozjs::jsapi::JSFunctionSpec::ZERO));

	parse_quote!(
		static #ident: &[::mozjs::jsapi::JSFunctionSpec] = &[
			#(#specs),*
		];
	)
}

pub(crate) fn properties_to_specs(properties: &[Property], accessors: &HashMap<String, Accessor>, class: &Ident, stat: bool) -> ItemStatic {
	let ident: Ident = if stat {
		parse_quote!(STATIC_PROPERTIES)
	} else {
		parse_quote!(PROPERTIES)
	};

	let mut specs: Vec<_> = properties.iter().map(|property| property.to_spec(class.clone())).collect();
	accessors
		.iter()
		.for_each(|(name, accessor)| specs.push(accessor.to_spec(Ident::new(name, class.span()))));

	specs.push(quote!(::mozjs::jsapi::JSPropertySpec::ZERO));

	parse_quote!(
		static #ident: &[::mozjs::jsapi::JSPropertySpec] = &[
			#(#specs),*
		];
	)
}

pub(crate) fn class_initialiser(class_ident: &Ident, constructor_ident: &Ident, constructor_nargs: u32) -> ItemImpl {
	let krate = quote!(::ion);
	let name_str = LitStr::new(&class_ident.to_string(), class_ident.span());

	parse2(quote!(
		impl #krate::ClassInitialiser for #class_ident {
			const NAME: &'static str = #name_str;

			fn class() -> &'static ::mozjs::jsapi::JSClass {
				&CLASS
			}

			fn constructor() -> (::ion::NativeFunction, ::core::primitive::u32) {
				(#constructor_ident, #constructor_nargs)
			}

			fn functions() -> &'static [::mozjs::jsapi::JSFunctionSpec] {
				&FUNCTIONS
			}

			fn properties() -> &'static [::mozjs::jsapi::JSPropertySpec] {
				&PROPERTIES
			}

			fn static_functions() -> &'static [::mozjs::jsapi::JSFunctionSpec] {
				&STATIC_FUNCTIONS
			}

			fn static_properties() -> &'static [::mozjs::jsapi::JSPropertySpec] {
				&STATIC_PROPERTIES
			}
		}
	))
	.unwrap()
}
