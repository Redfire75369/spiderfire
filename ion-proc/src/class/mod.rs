/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use quote::ToTokens;
use syn::{Error, ImplItem, Item, ItemFn, ItemImpl, ItemMod, LitStr, Meta, parse2, Result, Visibility};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::class::accessor::{flatten_accessors, get_accessor_name, impl_accessor, insert_accessor, insert_property_accessors};
use crate::class::attribute::{ClassAttribute, MethodAttribute};
use crate::class::automatic::{from_value, no_constructor, to_value};
use crate::class::constructor::impl_constructor;
use crate::class::method::{impl_method, Method, MethodKind, MethodReceiver};
use crate::class::operations::{class_finalise, class_ops, class_trace};
use crate::class::property::Property;
use crate::class::statics::{class_initialiser, class_spec, methods_to_specs, properties_to_specs};
use crate::utils::extract_last_argument;

pub(crate) mod accessor;
pub(crate) mod attribute;
pub(crate) mod automatic;
pub(crate) mod constructor;
pub(crate) mod method;
pub(crate) mod operations;
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

	let mut has_clone = false;
	let mut has_trace = false;

	let mut content_to_remove = Vec::new();
	for (i, item) in (**content).iter().enumerate() {
		content_to_remove.push(i);
		match item {
			Item::Struct(str) if class.is_none() => class = Some(str.clone()),
			Item::Impl(imp) if implementation.is_none() => {
				let impl_ty = extract_last_argument(&imp.self_ty);
				if imp.trait_.is_none() && impl_ty.is_some() && impl_ty.as_ref() == class.as_ref().map(|c| &c.ident) {
					let mut impl_items_to_remove = Vec::new();
					let mut impl_items_to_add = Vec::new();
					let mut imp = imp.clone();

					for (j, item) in (*imp.items).iter().enumerate() {
						match item {
							ImplItem::Fn(method) => {
								let mut method: ItemFn = parse2(method.to_token_stream())?;
								match &method.vis {
									Visibility::Public(_) => (),
									_ => continue,
								}

								let mut name = None;
								let mut names = vec![];
								let mut indexes = Vec::new();

								let mut kind = None;
								let mut internal = None;

								for (index, attr) in method.attrs.iter().enumerate() {
									if attr.path().is_ident("ion") {
										let args: Punctuated<MethodAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

										for arg in args {
											kind = kind.or_else(|| arg.to_kind());
											match arg {
												MethodAttribute::Skip(_) => internal = Some(index),
												MethodAttribute::Name(name_) => name = Some(name_.literal),
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

								let name = name.unwrap_or_else(|| LitStr::new(&method.sig.ident.to_string(), method.sig.ident.span()));
								names.insert(0, name);

								impl_items_to_remove.push(j);
								if let Some(internal) = internal {
									method.attrs.remove(internal);
									impl_items_to_add.push(ImplItem::Fn(parse2(method.to_token_stream())?));
								} else {
									for index in indexes {
										method.attrs.remove(index);
									}

									match kind {
										Some(MethodKind::Constructor) => {
											let (cons, _) = impl_constructor(method.clone(), &imp.self_ty)?;
											constructor = Some(Method { names, ..cons });
										}
										Some(MethodKind::Getter) => {
											let name = get_accessor_name(&method.sig.ident, false);
											let (getter, parameters) = impl_accessor(&method, &imp.self_ty, false, false)?;
											let getter = Method { names, ..getter };

											if parameters.this.is_some() {
												insert_accessor(&mut accessors, name, Some(getter), None);
											} else {
												insert_accessor(&mut static_accessors, name, Some(getter), None);
											}
										}
										Some(MethodKind::Setter) => {
											let name = get_accessor_name(&method.sig.ident, true);
											let (setter, parameters) = impl_accessor(&method, &imp.self_ty, false, true)?;
											let setter = Method { names, ..setter };

											if parameters.this.is_some() {
												insert_accessor(&mut accessors, name, None, Some(setter));
											} else {
												insert_accessor(&mut static_accessors, name, None, Some(setter));
											}
										}
										None => {
											let (method, _) = impl_method(method.clone(), &imp.self_ty, false, |_| Ok(()))?;
											let method = Method { names, ..method };

											if method.receiver == MethodReceiver::Dynamic {
												methods.push(method);
											} else {
												static_methods.push(method);
											}
										}
										Some(MethodKind::Internal) => continue,
									}
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
					for item in impl_items_to_add {
						imp.items.push(item);
					}

					implementation = Some(imp);
				} else if imp.trait_.as_ref().map(|tr| &tr.1) == Some(&parse_quote!(Clone)) {
					has_clone = true;
				} else if imp.trait_.as_ref().map(|tr| &tr.1) == Some(&parse_quote!(Traceable)) {
					has_trace = true;
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

	let mut class = if let Some(class) = class {
		class
	} else {
		return Err(Error::new(module.span(), "Expected Struct within Module"));
	};

	let has_clone = has_clone || (*class.attrs).iter().any(|attr| {
		if attr.path().is_ident("derive") {
			let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated).unwrap();
			return nested.iter().any(|meta| meta.path().is_ident("Clone"));
		}
		false
	});

	let mut class_name = None;
	let mut has_constructor = true;
	let mut impl_from_value = false;
	let mut impl_to_value = false;
	let mut impl_into_value = false;

	let mut class_attrs_to_remove = Vec::new();
	for (index, attr) in (*class.attrs).iter().enumerate() {
		if attr.path().is_ident("ion") {
			let args: Punctuated<ClassAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

			for arg in args {
				match arg {
					ClassAttribute::Name(name) => class_name = Some(name.literal),
					ClassAttribute::NoConstructor(_) => has_constructor = false,
					ClassAttribute::FromValue(_) => impl_from_value = true,
					ClassAttribute::ToValue(_) => impl_to_value = true,
					ClassAttribute::IntoValue(_) => impl_into_value = true,
				}
			}
			class_attrs_to_remove.push(index);
		}
	}

	class_attrs_to_remove.reverse();
	for index in class_attrs_to_remove {
		class.attrs.remove(index);
	}

	insert_property_accessors(&mut accessors, &mut class)?;

	let constructor = if has_constructor {
		if let Some(constructor) = constructor {
			constructor
		} else {
			return Err(Error::new(module.span(), "Expected Constructor"));
		}
	} else if constructor.is_some() {
		return Err(Error::new(module.span(), "Expected No Constructor"));
	} else if let Some(implementation) = &implementation {
		no_constructor(&implementation.self_ty)
	} else {
		return Err(Error::new(module.span(), "Expected Implementation"));
	};

	let class_name = class_name.unwrap_or_else(|| LitStr::new(&class.ident.to_string(), class.ident.span()));
	let class_spec = class_spec(&class.ident, &class_name);

	let finalise_operation = class_finalise(&class.ident);
	let trace_operation = has_trace.then(|| class_trace(&class.ident));
	let class_ops = class_ops(has_trace);

	let method_specs = methods_to_specs(&methods, false);
	let static_method_specs = methods_to_specs(&static_methods, true);
	let property_specs = properties_to_specs(&[], &accessors, &class.ident, false);
	let static_property_specs = properties_to_specs(&static_properties, &static_accessors, &class.ident, true);

	let from_value = if impl_from_value {
		if has_clone {
			Some(from_value(&class.ident))
		} else {
			return Err(Error::new(class.span(), "Expected Clone for Automatic FromValue Implementation"));
		}
	} else {
		None
	};
	let to_value = if impl_to_value {
		if has_clone {
			Some(to_value(&class.ident, true))
		} else {
			return Err(Error::new(class.span(), "Expected Clone for Automatic ToValue Implementation"));
		}
	} else if impl_into_value {
		Some(to_value(&class.ident, false))
	} else {
		None
	};
	let class_initialiser = class_initialiser(&class.ident, &constructor.method.sig.ident, constructor.nargs as u32);

	let accessors = flatten_accessors(accessors);
	let static_accessors = flatten_accessors(static_accessors);

	content.push(Item::Struct(class));

	if let Some(mut implementation) = implementation {
		add_methods(content, &mut implementation, vec![constructor]);
		add_methods(content, &mut implementation, methods);
		add_methods(content, &mut implementation, static_methods);
		add_methods(content, &mut implementation, accessors);
		add_methods(content, &mut implementation, static_accessors);
		content.push(Item::Impl(implementation));
	}

	content.push(Item::Fn(finalise_operation));
	if let Some(trace_operation) = trace_operation {
		content.push(Item::Fn(trace_operation))
	}
	content.push(Item::Static(class_ops));
	content.push(Item::Static(class_spec));
	content.push(Item::Static(method_specs));
	content.push(Item::Static(property_specs));
	content.push(Item::Static(static_method_specs));
	content.push(Item::Static(static_property_specs));

	if let Some(from_value) = from_value {
		content.push(Item::Impl(from_value));
	}
	if let Some(to_value) = to_value {
		content.push(Item::Impl(to_value));
	}
	content.push(Item::Impl(class_initialiser));

	Ok(module)
}

fn add_methods(content: &mut Vec<Item>, imp: &mut ItemImpl, methods: Vec<Method>) {
	for method in methods {
		content.push(Item::Fn(method.method));
		if let Some(inner) = method.inner {
			imp.items.push(ImplItem::Fn(parse2(inner.to_token_stream()).unwrap()))
		}
	}
}
