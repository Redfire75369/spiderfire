/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Error, ImplItem, Item, ItemFn, ItemMod, parse, Result, Visibility};
use syn::spanned::Spanned;

use crate::class::accessor::{flatten_accessors, get_accessor_name, impl_accessor, insert_accessor};
use crate::class::constructor::impl_constructor;
use crate::class::method::impl_method;
use crate::class::property::Property;
use crate::class::statics::{class_initialiser, class_spec, methods_to_specs, properties_to_specs};

pub(crate) mod accessor;
pub(crate) mod constructor;
pub(crate) mod method;
pub(crate) mod property;
pub(crate) mod statics;

pub(crate) type Accessor = (Option<ItemFn>, Option<ItemFn>);

pub(crate) fn impl_js_class(mut module: ItemMod) -> Result<TokenStream> {
	let content = &mut module.content.as_mut().unwrap().1;

	let mut class = None;
	let mut constructor = None;
	let mut regular_impl = None;
	let mut methods = Vec::new();
	let mut static_methods = Vec::new();
	let mut accessors = HashMap::new();
	let mut static_properties = Vec::new();
	let mut static_accessors = HashMap::new();

	let mut content_to_remove = Vec::new();
	for (i, item) in content.iter().enumerate() {
		content_to_remove.push(i);
		match item {
			Item::Struct(str) if class.is_none() => class = Some(str.clone()),
			Item::Impl(imp) if regular_impl.is_none() => {
				let object = class.as_ref().map(|o| {
					let ident = o.ident.clone();
					parse_quote!(#ident)
				});
				if Some(&*imp.self_ty) == object.as_ref() {
					let mut impl_items_to_remove = Vec::new();
					let mut imp = imp.clone();

					for (j, item) in imp.items.iter().enumerate() {
						impl_items_to_remove.push(j);
						match item {
							ImplItem::Method(method) => {
								let mut method: ItemFn = parse(method.to_token_stream().into())?;
								let mut indices = (None, None, None);

								for (i, attr) in method.attrs.iter().enumerate() {
									if attr == &parse_quote!(#[constructor]) {
										indices.0 = Some(i);
									} else if attr == &parse_quote!(#[get]) {
										indices.1 = Some(i);
									} else if attr == &parse_quote!(#[set]) {
										indices.2 = Some(i);
									}
								}

								if let Some(index) = indices.0 {
									method.attrs.remove(index);
									constructor = Some(impl_constructor(method.clone())?);
									continue;
								}

								if let Some(index) = indices.1 {
									method.attrs.remove(index);
									let name = get_accessor_name(&method.sig.ident, false);
									let (getter, has_this) = impl_accessor(&method, false)?;

									if has_this {
										insert_accessor(&mut accessors, name, Some(getter), None);
									} else {
										insert_accessor(&mut static_accessors, name, Some(getter), None);
									}
									continue;
								}

								if let Some(index) = indices.2 {
									method.attrs.remove(index);
									let name = get_accessor_name(&method.sig.ident, true);
									let (setter, has_this) = impl_accessor(&method, true)?;

									if has_this {
										insert_accessor(&mut accessors, name, None, Some(setter));
									} else {
										insert_accessor(&mut static_accessors, name, None, Some(setter));
									}
									continue;
								}

								let (method, nargs, this) = impl_method(method.clone(), |_| Ok(()))?;

								if this.is_some() {
									methods.push((method, nargs));
								} else {
									static_methods.push((method, nargs));
								}
							}
							ImplItem::Const(con) => {
								impl_items_to_remove.pop();
								if let Visibility::Public(_) = con.vis {
									if let Some(property) = Property::from_const(&con) {
										static_properties.push(property);
									}
								}
							}
							_ => {
								impl_items_to_remove.pop();
							}
						}
					}

					impl_items_to_remove.reverse();
					for index in impl_items_to_remove {
						imp.items.remove(index);
					}

					regular_impl = Some(imp);
				} else {
					content_to_remove.pop();
				}
			}
			_ => {
				content_to_remove.pop();
			}
		}
	}

	content_to_remove.reverse();
	for index in content_to_remove {
		content.remove(index);
	}

	if class.is_none() {
		return Err(Error::new(module.span(), "Expected Struct within Module"));
	}
	let class = class.unwrap();

	if constructor.is_none() {
		return Err(Error::new(module.span(), "Expected Constructor within Module"));
	}
	let regular_impl = regular_impl.unwrap();
	let (constructor, constructor_nargs) = constructor.unwrap();

	let class_spec = class_spec(&class);

	let method_specs = methods_to_specs(&methods, false);
	let static_method_specs = methods_to_specs(&static_methods, true);
	let property_specs = properties_to_specs(&[], &accessors, &class.ident, false);
	let static_property_specs = properties_to_specs(&static_properties, &static_accessors, &class.ident, true);

	let class_initialiser = class_initialiser(class.ident.clone(), (constructor.sig.ident.clone(), constructor_nargs as u32));

	let methods: Vec<_> = methods.into_iter().map(|(m, _)| m).collect();
	let static_methods: Vec<_> = static_methods.into_iter().map(|(m, _)| m).collect();
	let accessors = flatten_accessors(accessors);
	let static_accessors: Vec<_> = flatten_accessors(static_accessors);

	content.push(Item::Struct(class));
	content.push(Item::Impl(regular_impl));
	content.push(Item::Fn(constructor));
	for method in methods {
		content.push(Item::Fn(method));
	}
	for accessor in accessors {
		content.push(Item::Fn(accessor));
	}
	for method in static_methods {
		content.push(Item::Fn(method));
	}
	for accessor in static_accessors {
		content.push(Item::Fn(accessor));
	}

	content.push(Item::Static(class_spec));
	content.push(Item::Static(method_specs));
	content.push(Item::Static(property_specs));
	content.push(Item::Static(static_method_specs));
	content.push(Item::Static(static_property_specs));
	content.push(Item::Impl(class_initialiser));

	Ok(module.to_token_stream())
}
