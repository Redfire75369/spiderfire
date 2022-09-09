/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use proc_macro2::{Ident, TokenStream};
use syn::{Error, Expr, Field, Fields, ItemFn, ItemStruct, LitStr, Result, Type, Visibility};

use crate::class::method::impl_method;
use crate::function::parameters::Parameter;

#[derive(Debug)]
pub(crate) struct Accessor(Option<ItemFn>, Option<ItemFn>);

impl Accessor {
	pub(crate) fn from_field(field: &mut Field, class: Ident) -> Result<Option<Accessor>> {
		let krate = quote!(::ion);
		if let Visibility::Public(_) = field.vis {
			let ident = field.ident.as_ref().unwrap().clone();
			let ty = field.ty.clone();
			let mut convert = None;

			let mut index = None;
			for (i, attr) in field.attrs.iter().enumerate() {
				if attr.path == parse_quote!(convert) {
					index = Some(i);
					let convert_ty: Expr = attr.parse_args()?;
					convert = Some(convert_ty);
				}
			}
			if let Some(index) = index {
				field.attrs.remove(index);
			}
			let convert = convert.unwrap_or_else(|| parse_quote!(()));

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

			let (getter, _) = impl_accessor(&getter, false)?;
			let (setter, _) = impl_accessor(&setter, true)?;

			Ok(Some(Accessor(Some(getter), Some(setter))))
		} else {
			Ok(None)
		}
	}

	pub(crate) fn to_spec(&self, name: Ident) -> TokenStream {
		let krate = quote!(::ion);
		let Accessor(getter, setter) = self;
		if let Some(getter) = getter {
			let getter = getter.sig.ident.clone();
			let name = LitStr::new(&name.to_string(), name.span());
			if let Some(setter) = setter {
				let setter = setter.sig.ident.clone();
				quote!(#krate::property_spec_getter_setter!(#getter, #setter, #name, #krate::flags::PropertyFlags::CONSTANT_ENUMERATED))
			} else {
				quote!(#krate::property_spec_getter!(#getter, #name, #krate::flags::PropertyFlags::CONSTANT_ENUMERATED))
			}
		} else if let Some(setter) = setter {
			let setter = setter.sig.ident.clone();
			let name = LitStr::new(&name.to_string(), name.span());
			quote!(#krate::property_spec_setter!(#setter, #name, #krate::flags::PropertyFlags::CONSTANT_ENUMERATED))
		} else {
			let name = LitStr::new(&format!("{}\0", name), name.span());
			quote!(
				#krate::spec::create_property_spec_accessor(
					::std::concat!(#name, "\0"),
					::mozjs::jsapi::JSNativeWrapper { op: None, info: ::std::ptr::null_mut() },
					::mozjs::jsapi::JSNativeWrapper { op: None, info: ::std::ptr::null_mut() },
					#krate::flags::PropertyFlags::CONSTANT_ENUMERATED,
				)
			)
		}
	}
}

pub(crate) fn get_accessor_name(ident: &Ident, is_setter: bool) -> String {
	let mut name = ident.to_string();
	let pat = if is_setter { "set_" } else { "get_" };
	if name.starts_with(pat) {
		name.drain(0..4);
	}
	name
}

pub(crate) fn impl_accessor(method: &ItemFn, is_setter: bool) -> Result<(ItemFn, bool)> {
	let expected_args = if is_setter { 1 } else { 0 };
	let error_message = if is_setter {
		format!("Expected Setter to have {} argument", expected_args)
	} else {
		format!("Expected Getter to have {} arguments", expected_args)
	};
	let error = Error::new_spanned(&method.sig, error_message);
	let (accessor, _, this) = impl_method(method.clone(), |sig| {
		let params: Vec<_> = sig.inputs.iter().map(Parameter::from_arg).collect::<Result<_>>()?;
		let nargs = params.into_iter().fold(0, |mut acc, param| {
			if let Parameter::Normal(ty, _) = &param {
				if let Type::Path(_) = &*ty.ty {
					acc += 1;
				}
			}
			acc
		});
		(nargs == expected_args).then(|| ()).ok_or(error)
	})?;
	Ok((accessor, this.is_some()))
}

pub(crate) fn insert_accessor(accessors: &mut HashMap<String, Accessor>, name: String, getter: Option<ItemFn>, setter: Option<ItemFn>) {
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

pub(crate) fn flatten_accessors(accessors: HashMap<String, Accessor>) -> Vec<ItemFn> {
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
