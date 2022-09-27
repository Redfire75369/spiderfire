/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use quote::ToTokens;
use syn::{Error, ImplItem, Item, ItemFn, ItemImpl, ItemMod, parse2, Result, Visibility};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::class::accessor::{flatten_accessors, get_accessor_name, impl_accessor, insert_accessor, insert_property_accessors};
use crate::class::attribute::MethodAttribute;
use crate::class::constructor::impl_constructor;
use crate::class::method::{impl_method, Method, MethodKind, MethodReceiver};
use crate::class::property::Property;
use crate::class::statics::{class_initialiser, class_spec, into_js_val, methods_to_specs, properties_to_specs};

pub(crate) mod accessor;
pub(crate) mod attribute;
pub(crate) mod constructor;
pub(crate) mod method;
pub(crate) mod property;
pub(crate) mod statics;

pub(crate) fn impl_js_class(mut module: ItemMod) -> Result<ItemMod> {
	let content = &mut module.content.as_mut().unwrap().1;

	let mut class = None;
	let mut constructor = None;
	let mut implementation = None;
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
			Item::Impl(imp) if implementation.is_none() => {
				let object = class.as_ref().map(|o| {
					let ident = o.ident.clone();
					parse_quote!(#ident)
				});
				if Some(&*imp.self_ty) == object.as_ref() {
					let object = parse2(object.unwrap().to_token_stream())?;

					let mut impl_items_to_remove = Vec::new();
					let mut imp = imp.clone();

					for (j, item) in imp.items.iter().enumerate() {
						match item {
							ImplItem::Method(method) => {
								let mut method: ItemFn = parse2(method.to_token_stream())?;
								match &method.vis {
									Visibility::Public(_) => (),
									_ => continue,
								}

								let mut names = vec![method.sig.ident.clone()];
								let mut indexes = Vec::new();

								let mut kind = None;
								let mut internal = false;

								for (index, attr) in method.attrs.iter().enumerate() {
									if attr.path.is_ident("ion") {
										let args: Punctuated<MethodAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

										for arg in args {
											kind = kind.or(arg.to_kind());
											match arg {
												MethodAttribute::Internal(_) => {
													internal = true;
												}
												MethodAttribute::Alias(alias) => {
													for alias in alias.aliases {
														names.push(alias);
													}
												}
												_ => (),
											}
										}
										indexes.push(index);
									}
								}

								if !internal {
									for index in indexes {
										method.attrs.remove(index);
									}

									match kind {
										Some(MethodKind::Constructor) => {
											let (cons, _) = impl_constructor(method.clone(), &object)?;
											constructor = Some(Method { aliases: names, ..cons });
										}
										Some(MethodKind::Getter) => {
											let name = get_accessor_name(&method.sig.ident, false);
											let (getter, parameters) = impl_accessor(&method, &object, false, false)?;
											let getter = Method { aliases: names, ..getter };

											if parameters.this.is_some() {
												insert_accessor(&mut accessors, name, Some(getter), None);
											} else {
												insert_accessor(&mut static_accessors, name, Some(getter), None);
											}
										}
										Some(MethodKind::Setter) => {
											let name = get_accessor_name(&method.sig.ident, true);
											let (setter, parameters) = impl_accessor(&method, &object, false, true)?;
											let setter = Method { aliases: names, ..setter };

											if parameters.this.is_some() {
												insert_accessor(&mut accessors, name, None, Some(setter));
											} else {
												insert_accessor(&mut static_accessors, name, None, Some(setter));
											}
										}
										None => {
											let (method, _) = impl_method(method.clone(), &object, false, |_| Ok(()))?;
											let method = Method { aliases: names, ..method };

											if method.receiver == MethodReceiver::Dynamic {
												methods.push(method);
											} else {
												static_methods.push(method);
											}
										}
										Some(MethodKind::Internal) => continue,
									}
									impl_items_to_remove.push(j);
								}
							}
							ImplItem::Const(con) => {
								if let Visibility::Public(_) = con.vis {
									if let Some(property) = Property::from_const(con) {
										static_properties.push(property);
									}
								}
							}
							_ => (),
						}
					}

					impl_items_to_remove.reverse();
					for index in impl_items_to_remove {
						imp.items.remove(index);
					}

					implementation = Some(imp);
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
	let mut class = class.unwrap();

	insert_property_accessors(&mut accessors, &mut class)?;

	if constructor.is_none() {
		return Err(Error::new(module.span(), "Expected Constructor within Module"));
	}

	let constructor = constructor.unwrap();

	let class_spec = class_spec(&class);

	let method_specs = methods_to_specs(&methods, false);
	let static_method_specs = methods_to_specs(&static_methods, true);
	let property_specs = properties_to_specs(&[], &accessors, &class.ident, false);
	let static_property_specs = properties_to_specs(&static_properties, &static_accessors, &class.ident, true);

	let into_js_val = into_js_val(class.ident.clone());
	let class_initialiser = class_initialiser(class.ident.clone(), &constructor);

	let accessors = flatten_accessors(accessors);
	let static_accessors = flatten_accessors(static_accessors);

	content.push(Item::Struct(class));

	if let Some(mut regular_impl) = implementation {
		add_methods(content, &mut regular_impl, vec![constructor]);
		add_methods(content, &mut regular_impl, methods);
		add_methods(content, &mut regular_impl, static_methods);
		add_methods(content, &mut regular_impl, accessors);
		add_methods(content, &mut regular_impl, static_accessors);
		content.push(Item::Impl(regular_impl));
	}

	content.push(Item::Static(class_spec));
	content.push(Item::Static(method_specs));
	content.push(Item::Static(property_specs));
	content.push(Item::Static(static_method_specs));
	content.push(Item::Static(static_property_specs));

	content.push(Item::Impl(into_js_val));
	content.push(Item::Impl(class_initialiser));

	Ok(module)
}

fn add_methods(content: &mut Vec<Item>, imp: &mut ItemImpl, methods: Vec<Method>) {
	for method in methods {
		content.push(Item::Fn(method.method));
		if let Some(inner) = method.inner {
			imp.items.push(ImplItem::Method(parse2(inner.to_token_stream()).unwrap()))
		}
	}
}
