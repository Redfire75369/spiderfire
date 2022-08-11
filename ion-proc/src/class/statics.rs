/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use proc_macro2::Ident;
use syn::{ItemFn, ItemImpl, ItemStatic, ItemStruct, LitStr};

use crate::class::accessor::Accessor;
use crate::class::property::Property;

pub(crate) fn class_spec(object: &ItemStruct) -> ItemStatic {
	let krate = quote!(::ion);
	let name = LitStr::new(&format!("{}\0", object.ident), object.ident.span());

	parse_quote!(
		static CLASS: ::mozjs::jsapi::JSClass = ::mozjs::jsapi::JSClass {
			name: #name.as_ptr() as *const i8,
			flags: #krate::class_reserved_slots(0),
			cOps: ::std::ptr::null_mut(),
			spec: ::std::ptr::null_mut(),
			ext: ::std::ptr::null_mut(),
			oOps: ::std::ptr::null_mut(),
		};
	)
}

pub(crate) fn methods_to_specs(methods: &[(ItemFn, usize)], stat: bool) -> ItemStatic {
	let krate = quote!(::ion);
	let ident = if stat { quote!(STATIC_FUNCTIONS) } else { quote!(FUNCTIONS) };
	let mut specs: Vec<_> = methods
		.into_iter()
		.map(|(method, nargs)| {
			let name = LitStr::new(&method.sig.ident.to_string(), method.sig.ident.span());
			let ident = method.sig.ident.clone();
			let nargs = *nargs as u16;
			quote!(#krate::function_spec!(#ident, #name, #nargs, #krate::flags::PropertyFlags::CONSTANT))
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

	let mut specs: Vec<_> = properties.into_iter().map(|property| property.to_spec(class.clone())).collect();
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

pub(crate) fn class_initialiser(name: Ident, constructor: (Ident, u32)) -> ItemImpl {
	let krate = quote!(::ion);
	let (ident, nargs) = constructor;
	parse_quote!(
		impl #krate::ClassInitialiser for #name {
			fn class() -> &'static ::mozjs::jsapi::JSClass {
				&CLASS
			}

			fn constructor() -> (::ion::NativeFunction, u32) {
				(#ident, #nargs)
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
	)
}
