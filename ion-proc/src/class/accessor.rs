/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use syn::{Error, Field, Fields, ItemFn, ItemStruct, LitStr, Result, Type, Visibility};
use syn::punctuated::Punctuated;

use crate::class::attribute::PropertyAttribute;
use crate::class::method::{impl_method, Method};
use crate::function::parameters::{Parameter, Parameters};

#[derive(Debug)]
pub(crate) struct Accessor(Option<Method>, Option<Method>);

impl Accessor {
	pub(crate) fn from_field(field: &mut Field, class: Ident) -> Result<Option<Accessor>> {
		let krate = quote!(::ion);
		if let Visibility::Public(_) = field.vis {
			let ident = field.ident.as_ref().unwrap().clone();
			let ty = field.ty.clone();
			let mut conversion = None;

			let mut names = vec![ident.clone()];
			let mut indexes = Vec::new();
			for (index, attr) in field.attrs.iter().enumerate() {
				if attr.path.is_ident("ion") {
					let args: Punctuated<PropertyAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

					for arg in args {
						match arg {
							PropertyAttribute::Convert { conversion: conversion_expr, .. } => {
								conversion = conversion.or(Some(conversion_expr));
							}
							PropertyAttribute::Alias(alias) => {
								for alias in alias.aliases {
									names.push(alias);
								}
							}
						}
					}
					indexes.push(index);
				}
			}
			indexes.reverse();
			for index in indexes {
				field.attrs.remove(index);
			}
			let convert = conversion.unwrap_or_else(|| parse_quote!(()));

			let getter_ident = Ident::new(&format!("get_{}", ident), ident.span());
			let setter_ident = Ident::new(&format!("set_{}", ident), ident.span());

			let getter = parse_quote!(
				fn #getter_ident(#[this] this: &#class) -> #krate::Result<#ty> {
					Ok(this.#ident)
				}
			);
			let setter = parse_quote!(
				fn #setter_ident(#[this] this: &mut #class, #[convert(#convert)] #ident: #ty) -> #krate::Result<()> {
					this.#ident = #ident;
					Ok(())
				}
			);

			let (mut getter, _) = impl_accessor(&getter, &class, true, false)?;
			let (mut setter, _) = impl_accessor(&setter, &class, true, true)?;
			getter.aliases = names.clone();
			setter.aliases = names;

			Ok(Some(Accessor(Some(getter), Some(setter))))
		} else {
			Ok(None)
		}
	}

	pub(crate) fn to_spec(&self, name: Ident) -> TokenStream {
		let krate = quote!(::ion);

		match self {
			Accessor(Some(getter), Some(setter)) => {
				let getter = getter.method.sig.ident.clone();
				let setter = setter.method.sig.ident.clone();

				let name = LitStr::new(&name.to_string(), name.span());
				quote!(#krate::property_spec_getter_setter!(#getter, #setter, #name, #krate::flags::PropertyFlags::CONSTANT_ENUMERATED))
			}
			Accessor(Some(getter), None) => {
				let getter = getter.method.sig.ident.clone();
				let name = LitStr::new(&name.to_string(), name.span());
				quote!(#krate::property_spec_getter!(#getter, #name, #krate::flags::PropertyFlags::CONSTANT_ENUMERATED))
			}
			Accessor(None, Some(setter)) => {
				let setter = setter.method.sig.ident.clone();
				let name = LitStr::new(&name.to_string(), name.span());
				quote!(#krate::property_spec_setter!(#setter, #name, #krate::flags::PropertyFlags::CONSTANT_ENUMERATED))
			}
			Accessor(None, None) => {
				let name = LitStr::new(&format!("{}\0", name), name.span());
				quote!(
					#krate::spec::create_property_spec_accessor(
						#name,
						::mozjs::jsapi::JSNativeWrapper { op: None, info: ::std::ptr::null_mut() },
						::mozjs::jsapi::JSNativeWrapper { op: None, info: ::std::ptr::null_mut() },
						#krate::flags::PropertyFlags::CONSTANT_ENUMERATED,
					)
				)
			}
		}
	}
}

pub(crate) fn get_accessor_name(ident: &Ident, is_setter: bool) -> String {
	let mut name = ident.to_string();
	let pat_snake = if is_setter { "set_" } else { "get_" };
	let pat_camel = if is_setter { "set" } else { "get" };
	if name.starts_with(pat_snake) {
		name.drain(0..4);
		if name.is_case(Case::Snake) {
			name = name.to_case(Case::Camel);
		}
	} else if name.starts_with(pat_camel) {
		name.drain(0..3);
		if name.is_case(Case::Pascal) {
			name = name.to_case(Case::Camel);
		}
	}
	name
}

pub(crate) fn impl_accessor(method: &ItemFn, ident: &Ident, keep_inner: bool, is_setter: bool) -> Result<(Method, Parameters)> {
	let expected_args = if is_setter { 1 } else { 0 };
	let error_message = if is_setter {
		format!("Expected Setter to have {} argument", expected_args)
	} else {
		format!("Expected Getter to have {} arguments", expected_args)
	};
	let error = Error::new_spanned(&method.sig, error_message);
	let (accessor, parameters) = impl_method(method.clone(), ident, keep_inner, |sig| {
		let parameters = Parameters::parse(&sig.inputs, Some(ident), true)?;
		let nargs = parameters.parameters.iter().fold(0, |mut acc, param| {
			if let Parameter::Regular { ty, .. } = &param {
				if let Type::Path(_) = &**ty {
					acc += 1;
				}
			}
			acc
		});
		(nargs == expected_args).then(|| ()).ok_or(error)
	})?;

	Ok((accessor, parameters))
}

pub(crate) fn insert_accessor(accessors: &mut HashMap<String, Accessor>, name: String, getter: Option<Method>, setter: Option<Method>) {
	match accessors.entry(name) {
		Entry::Occupied(mut o) => match (getter, setter) {
			(Some(g), Some(s)) => *o.get_mut() = Accessor(Some(g), Some(s)),
			(Some(g), None) => o.get_mut().0 = Some(g),
			(None, Some(s)) => o.get_mut().1 = Some(s),
			(None, None) => {}
		},
		Entry::Vacant(v) => {
			v.insert(Accessor(getter, setter));
		}
	}
}

pub(crate) fn flatten_accessors(accessors: HashMap<String, Accessor>) -> Vec<Method> {
	accessors
		.into_iter()
		.flat_map(|(_, Accessor(getter, setter))| [getter, setter])
		.flatten()
		.collect()
}

pub(crate) fn insert_property_accessors(accessors: &mut HashMap<String, Accessor>, class: &mut ItemStruct) -> Result<()> {
	if let Fields::Named(fields) = &mut class.fields {
		return fields.named.iter_mut().try_for_each(|field| {
			if let Some(accessor) = Accessor::from_field(field, class.ident.clone())? {
				accessors.insert(field.ident.as_ref().unwrap().to_string(), accessor);
			}
			Ok(())
		});
	}
	Ok(())
}
