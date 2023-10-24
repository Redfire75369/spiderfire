/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Error, FnArg, ImplItem, ImplItemFn, ItemFn, ItemImpl, parse2, Result, Type, Visibility};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::attribute::class::{MethodAttribute, Name};
use crate::attribute::krate::crate_from_attributes;
use crate::class::accessor::{get_accessor_name, impl_accessor, insert_accessor};
use crate::class::constructor::impl_constructor;
use crate::class::method::{impl_method, Method, MethodKind, MethodReceiver};
use crate::class::property::Property;
use crate::class::r#impl::spec::PrototypeSpecs;

mod spec;

pub(super) fn impl_js_class_impl(r#impl: &mut ItemImpl) -> Result<[ItemImpl; 2]> {
	let ion = &crate_from_attributes(&r#impl.attrs);

	if !r#impl.generics.params.is_empty() {
		return Err(Error::new(r#impl.generics.span(), "Native Class Impls cannot have generics."));
	}

	if let Some(r#trait) = &r#impl.trait_ {
		return Err(Error::new(r#trait.1.span(), "Native Class Impls cannot be for a trait."));
	}

	let r#type = *r#impl.self_ty.clone();
	let mut constructor: Option<Method> = None;
	let mut specs = PrototypeSpecs::default();

	for item in &mut r#impl.items {
		match item {
			ImplItem::Const(r#const) => {
				if let Some((property, r#static)) = Property::from_const(r#const)? {
					if r#static {
						specs.properties.1.push(property);
					} else {
						specs.properties.0.push(property);
					}
				}
			}
			ImplItem::Fn(r#fn) => {
				if let Some(parsed_constructor) = parse_class_method(ion, r#fn, &mut specs, &r#type)? {
					if let Some(constructor) = constructor.as_ref() {
						return Err(Error::new(
							r#fn.span(),
							format!(
								"Received multiple constructor implementations: {} and {}.",
								constructor.method.sig.ident, parsed_constructor.method.sig.ident
							),
						));
					} else {
						constructor = Some(parsed_constructor);
					}
				}
			}
			_ => (),
		}
	}

	let constructor = match constructor {
		Some(constructor) => constructor,
		None => return Err(Error::new(r#impl.span(), "Native Class Impls must contain a constructor.")),
	};

	let ident: Ident = parse2(quote_spanned!(r#type.span() => #r#type))?;
	class_definition(ion, r#impl.span(), &r#type, &ident, constructor, specs)
}

fn parse_class_method(ion: &TokenStream, r#fn: &mut ImplItemFn, specs: &mut PrototypeSpecs, r#type: &Type) -> Result<Option<Method>> {
	match &r#fn.vis {
		Visibility::Public(_) => (),
		_ => return Ok(None),
	}

	let mut name = None;
	let mut names = vec![];
	let mut indexes = Vec::new();

	let mut kind = None;
	let mut skip = None;

	for (index, attr) in r#fn.attrs.iter().enumerate() {
		if attr.path().is_ident("ion") {
			let args: Punctuated<MethodAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

			for arg in args {
				let arg_kind = arg.to_kind();
				if let Some(arg_kind) = arg_kind {
					match kind {
						Some(kind) => {
							return Err(Error::new(
								r#fn.span(),
								format!("Received conflicting method kinds: {:?} and {:?}", arg_kind, kind),
							));
						}
						None => kind = Some(arg_kind),
					}
				}
				match arg {
					MethodAttribute::Skip(_) => {
						skip = Some(index);
						break;
					}
					MethodAttribute::Name(name_) => name = Some(name_.name),
					MethodAttribute::Alias(alias) => {
						for alias in alias.aliases {
							names.push(Name::String(alias));
						}
					}
					_ => (),
				}
			}
			indexes.push(index);
		}
	}

	if let Some(skip) = skip {
		r#fn.attrs.remove(skip);
		return Ok(None);
	}
	for index in indexes {
		r#fn.attrs.remove(index);
	}

	let name = match name {
		Some(name) => name,
		None => {
			if kind == Some(MethodKind::Getter) || kind == Some(MethodKind::Setter) {
				Name::from_string(
					get_accessor_name(r#fn.sig.ident.to_string(), kind == Some(MethodKind::Setter)),
					r#fn.sig.ident.span(),
				)
			} else {
				Name::from_string(r#fn.sig.ident.to_string(), r#fn.sig.ident.span())
			}
		}
	};
	names.insert(0, name.clone());

	let method: ItemFn = parse2(r#fn.to_token_stream())?;

	for input in &mut r#fn.sig.inputs {
		let attrs = match input {
			FnArg::Receiver(arg) => &mut arg.attrs,
			FnArg::Typed(arg) => &mut arg.attrs,
		};
		attrs.clear();
	}

	match kind {
		Some(MethodKind::Constructor) => {
			let constructor = impl_constructor(ion, method, r#type)?;
			return Ok(Some(Method { names, ..constructor }));
		}
		Some(MethodKind::Getter) => {
			let (getter, parameters) = impl_accessor(ion, method, r#type, false)?;
			let getter = Method { names, ..getter };

			if parameters.this.is_some() {
				insert_accessor(&mut specs.accessors.0, name.as_string(), Some(getter), None);
			} else {
				insert_accessor(&mut specs.accessors.1, name.as_string(), Some(getter), None);
			}
		}
		Some(MethodKind::Setter) => {
			let (setter, parameters) = impl_accessor(ion, method, r#type, true)?;
			let setter = Method { names, ..setter };

			if parameters.this.is_some() {
				insert_accessor(&mut specs.accessors.0, name.as_string(), None, Some(setter));
			} else {
				insert_accessor(&mut specs.accessors.1, name.as_string(), None, Some(setter));
			}
		}
		None => {
			let (method, _) = impl_method(ion, method, r#type, |_| Ok(()))?;
			let method = Method { names, ..method };

			if method.receiver == MethodReceiver::Dynamic {
				specs.methods.0.push(method);
			} else {
				specs.methods.1.push(method);
			}
		}
	}

	Ok(None)
}

fn class_definition(
	ion: &TokenStream, span: Span, r#type: &Type, ident: &Ident, constructor: Method, specs: PrototypeSpecs,
) -> Result<[ItemImpl; 2]> {
	let spec_functions = specs.to_spec_functions(ion, span, ident)?.into_array();
	let constructor_function = constructor.method;
	let functions = specs.into_functions().into_iter().map(|method| method.method);

	let mut spec_impls: ItemImpl = parse2(quote_spanned!(span => impl #r#type {
		#constructor_function
		#(#functions)*
		#(#spec_functions)*
	}))?;
	spec_impls.attrs.push(parse_quote!(#[doc(hidden)]));

	let constructor_nargs = constructor.nargs as u32;
	let class_definition = parse2(quote_spanned!(span => impl #ion::ClassDefinition for #r#type {
		const NAME: &'static str = ::std::stringify!(#ident);

		fn class() -> &'static #ion::class::NativeClass {
			Self::__ion_native_class()
		}

		fn constructor() -> (#ion::functions::NativeFunction, ::core::primitive::u32) {
			(Self::__ion_bindings_constructor, #constructor_nargs)
		}

		fn functions() -> &'static [::mozjs::jsapi::JSFunctionSpec] {
			Self::__ion_function_specs()
		}

		fn properties() -> &'static [::mozjs::jsapi::JSPropertySpec] {
			Self::__ion_property_specs()
		}

		fn static_functions() -> &'static [::mozjs::jsapi::JSFunctionSpec] {
			Self::__ion_static_function_specs()
		}

		fn static_properties() -> &'static [::mozjs::jsapi::JSPropertySpec] {
			Self::__ion_static_property_specs()
		}
	}))?;
	Ok([spec_impls, class_definition])
}
