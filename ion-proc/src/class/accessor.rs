/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Error, Field, Fields, ItemFn, ItemStruct, LitStr, parse2, Result, Type, Visibility};
use syn::punctuated::Punctuated;

use crate::attribute::class::{FieldAttribute, Name};
use crate::class::method::{impl_method, Method};
use crate::function::parameters::{Parameter, Parameters};

#[derive(Debug)]
pub(crate) struct Accessor(pub(crate) Option<Method>, Option<Method>);

impl Accessor {
	pub(crate) fn from_field(field: &mut Field, class_ty: &Type) -> Result<Option<Accessor>> {
		let krate = quote!(::ion);
		if let Visibility::Public(_) = field.vis {
			let ident = field.ident.as_ref().unwrap().clone();
			let ty = field.ty.clone();
			let mut conversion = None;

			let mut name = None;
			let mut names = Vec::new();
			let mut readonly = false;
			let mut skip = false;

			let mut indexes = Vec::new();
			for (index, attr) in field.attrs.iter().enumerate() {
				if attr.path().is_ident("ion") {
					let args: Punctuated<FieldAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

					for arg in args {
						match arg {
							FieldAttribute::Name(name_) => name = Some(name_.name),
							FieldAttribute::Alias(alias) => {
								for alias in alias.aliases {
									names.push(Name::String(alias));
								}
							}
							FieldAttribute::Convert(convert) => conversion = conversion.or(Some(convert.conversion)),
							FieldAttribute::Readonly(_) => readonly = true,
							FieldAttribute::Skip(_) => skip = true,
						}
					}
					indexes.push(index);
				}
			}
			indexes.reverse();
			for index in indexes {
				field.attrs.remove(index);
			}

			if skip {
				return Ok(None);
			}

			match name {
				Some(name) => names.insert(0, name),
				None => names.insert(0, Name::from_string(get_accessor_name(ident.to_string(), !readonly), ident.span())),
			}

			let getter_ident = Ident::new(&format!("get_{}", ident), ident.span());
			let getter = parse2(quote!(
				fn #getter_ident(#[ion(this)] this: &#krate::Object) -> #krate::Result<#ty> {
					let self_ = <#class_ty as #krate::ClassInitialiser>::get_private(this);
					Ok(self_.#ident)
				}
			))
			.unwrap();
			let (mut getter, _) = impl_accessor(&getter, class_ty, true, false)?;
			getter.names = names.clone();

			if !readonly {
				let convert = conversion.unwrap_or_else(|| parse_quote!(()));
				let setter_ident = Ident::new(&format!("set_{}", ident), ident.span());

				let setter = parse2(quote!(
					fn #setter_ident(#[ion(this)] this: &mut #krate::Object, #[ion(convert = #convert)] #ident: #ty) -> #krate::Result<()> {
						let self_ = <#class_ty as #krate::ClassInitialiser>::get_private(this);
						self_.#ident = #ident;
						Ok(())
					}
				))
				.unwrap();
				let (mut setter, _) = impl_accessor(&setter, class_ty, true, true)?;
				setter.names = names;

				Ok(Some(Accessor(Some(getter), Some(setter))))
			} else {
				Ok(Some(Accessor(Some(getter), None)))
			}
		} else {
			Ok(None)
		}
	}

	pub(crate) fn to_specs(&self) -> Vec<TokenStream> {
		let krate = quote!(::ion);

		let names = self.0.as_ref().or(self.1.as_ref()).map(|method| &*method.names).unwrap_or_default();
		names
			.iter()
			.map(|name| {
				let mut function_ident = Ident::new("property_spec", Span::call_site());
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

				match self {
					Accessor(Some(getter), Some(setter)) => {
						let getter = getter.method.sig.ident.clone();
						let setter = setter.method.sig.ident.clone();

						function_ident = format_ident!("{}_getter_setter", function_ident);
						quote!(#krate::#function_ident!(#getter, #setter, #key, #flags))
					}
					Accessor(Some(getter), None) => {
						let getter = getter.method.sig.ident.clone();

						function_ident = format_ident!("{}_getter", function_ident);
						quote!(#krate::#function_ident!(#getter, #key, #flags))
					}
					Accessor(None, Some(setter)) => {
						let setter = setter.method.sig.ident.clone();
						function_ident = format_ident!("{}_getter", function_ident);
						quote!(#krate::#function_ident!(#setter, #key, #flags))
					}
					Accessor(None, None) => {
						function_ident = format_ident!("create_{}_accessor", function_ident);
						quote!(
							#krate::spec::#function_ident(
								#key,
								::mozjs::jsapi::JSNativeWrapper { op: None, info: ::std::ptr::null_mut() },
								::mozjs::jsapi::JSNativeWrapper { op: None, info: ::std::ptr::null_mut() },
								#flags,
							)
						)
					}
				}
			})
			.collect()
	}
}

pub(crate) fn get_accessor_name(mut name: String, is_setter: bool) -> String {
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

pub(crate) fn impl_accessor(method: &ItemFn, ty: &Type, keep_inner: bool, is_setter: bool) -> Result<(Method, Parameters)> {
	let expected_args = is_setter as i32;
	let error_message = if is_setter {
		format!("Expected Setter to have {} argument", expected_args)
	} else {
		format!("Expected Getter to have {} arguments", expected_args)
	};
	let error = Error::new_spanned(&method.sig, error_message);

	let (accessor, parameters) = impl_method(method.clone(), ty, keep_inner, |sig| {
		let parameters = Parameters::parse(&sig.inputs, Some(ty), false)?;
		let nargs = parameters.parameters.iter().fold(0, |mut acc, param| {
			if let Parameter::Regular { ty, .. } = &param {
				if let Type::Path(_) = &**ty {
					acc += 1;
				}
			}
			acc
		});
		(nargs == expected_args).then_some(()).ok_or(error)
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
	let ident = &class.ident;
	let ty = parse2(quote!(#ident))?;
	if let Fields::Named(fields) = &mut class.fields {
		return fields.named.iter_mut().try_for_each(|field| {
			if let Some(accessor) = Accessor::from_field(field, &ty)? {
				if let Some(getter) = accessor.0.as_ref() {
					insert_accessor(accessors, getter.names[0].as_string(), accessor.0.clone(), accessor.1.clone());
				} else if let Some(setter) = accessor.1.as_ref() {
					insert_accessor(accessors, setter.names[0].as_string(), accessor.0.clone(), accessor.1.clone());
				}
			}
			Ok(())
		});
	}
	Ok(())
}
