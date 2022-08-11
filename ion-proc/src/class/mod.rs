/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{Error, ImplItem, Item, ItemFn, ItemMod, ItemStatic, ItemStruct, LitStr, parse, Result, Visibility};
use syn::spanned::Spanned;

use crate::class::constructor::impl_constructor;
use crate::class::method::impl_method;
use crate::class::property::Property;

pub(crate) mod constructor;
pub(crate) mod method;
pub(crate) mod property;

pub(crate) fn impl_js_class(mut module: ItemMod) -> Result<TokenStream> {
	let krate = quote!(::ion);
	let content = &mut module.content.as_mut().unwrap().1;

	let mut object = None;
	let mut constructor = None;
	let mut regular_impl = None;
	let mut methods = Vec::new();
	let mut static_methods = Vec::new();
	let mut static_properties = Vec::new();

	let mut content_to_remove = Vec::new();
	for (i, item) in content.iter().enumerate() {
		content_to_remove.push(i);
		match item {
			Item::Struct(str) if object.is_none() => object = Some(str.clone()),
			Item::Impl(imp) => {
				let object = object.as_ref().map(|o| {
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
								let mut constructor_index = None;
								for (i, attr) in method.attrs.iter().enumerate() {
									if attr == &parse_quote!(#[constructor]) {
										constructor_index = Some(i);
									}
								}

								if let Some(index) = constructor_index {
									method.attrs.remove(index);
									constructor = Some(impl_constructor(method.clone())?);
									continue;
								}

								let (method, nargs, this) = impl_method(method.clone())?;

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

	if object.is_none() {
		return Err(Error::new(module.span(), "Expected Struct within Module"));
	}
	let object = object.unwrap();

	if constructor.is_none() {
		return Err(Error::new(module.span(), "Expected Constructor within Module"));
	}
	let (constructor, constructor_nargs) = constructor.unwrap();
	let constructor_ident = constructor.sig.ident.clone();
	let constructor_nargs = constructor_nargs as u32;

	let class = js_class(&object);
	let class_name = object.ident.clone();

	let methods_array = js_methods(&methods, false);
	let static_methods_array = js_methods(&static_methods, true);
	let static_properties_array = js_properties(static_properties, true, &class_name);

	let methods: Vec<_> = methods.into_iter().map(|(m, _)| m).collect();
	let static_methods: Vec<_> = static_methods.into_iter().map(|(m, _)| m).collect();

	let class_initialiser = parse_quote!(
		impl #krate::ClassInitialiser for #class_name {
			fn class() -> &'static ::mozjs::jsapi::JSClass {
				&CLASS
			}

			fn constructor() -> (::ion::NativeFunction, u32) {
				(#constructor_ident, #constructor_nargs)
			}

			fn functions() -> &'static [::mozjs::jsapi::JSFunctionSpec] {
				&FUNCTIONS
			}

			fn static_functions() -> &'static [::mozjs::jsapi::JSFunctionSpec] {
				&STATIC_FUNCTIONS
			}

			fn static_properties() -> &'static [::mozjs::jsapi::JSPropertySpec] {
				&STATIC_PROPERTIES
			}
		}
	);

	content.push(Item::Struct(object));
	if let Some(regular_impl) = regular_impl {
		content.push(Item::Impl(regular_impl));
	}
	content.push(Item::Fn(constructor));
	for method in methods {
		content.push(Item::Fn(method));
	}
	for method in static_methods {
		content.push(Item::Fn(method));
	}
	content.push(Item::Static(class));
	content.push(Item::Static(methods_array));
	content.push(Item::Static(static_methods_array));
	content.push(Item::Static(static_properties_array));
	content.push(Item::Impl(class_initialiser));

	Ok(module.to_token_stream())
}

pub(crate) fn js_class(object: &ItemStruct) -> ItemStatic {
	let krate = quote!(::ion);
	let name = format!("{}\0", object.ident);
	let name = LitStr::new(&name, object.ident.span());

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

pub(crate) fn js_methods(methods: &[(ItemFn, usize)], stat: bool) -> ItemStatic {
	let krate = quote!(::ion);
	let ident: Ident = if stat { parse_quote!(STATIC_FUNCTIONS) } else { parse_quote!(FUNCTIONS) };
	let specs: Vec<_> = methods
		.into_iter()
		.map(|(method, nargs)| {
			let name = LitStr::new(&method.sig.ident.to_string(), method.sig.ident.span());
			let ident = method.sig.ident.clone();
			let nargs = *nargs as u16;
			quote!(#krate::function_spec!(#ident, #name, #nargs, #krate::flags::PropertyFlags::CONSTANT))
		})
		.collect();

	if specs.is_empty() {
		parse_quote!(
			static #ident: &[::mozjs::jsapi::JSFunctionSpec] = &[
				::mozjs::jsapi::JSFunctionSpec::ZERO,
			];
		)
	} else {
		parse_quote!(
			static #ident: &[::mozjs::jsapi::JSFunctionSpec] = &[
				#(#specs),*,
				::mozjs::jsapi::JSFunctionSpec::ZERO,
			];
		)
	}
}

pub(crate) fn js_properties(properties: Vec<Property>, stat: bool, class: &Ident) -> ItemStatic {
	let ident: Ident = if stat {
		parse_quote!(STATIC_PROPERTIES)
	} else {
		parse_quote!(PROPERTIES)
	};

	let specs: Vec<_> = properties.into_iter().map(|property| property.into_spec(class.clone())).collect();
	if specs.is_empty() {
		parse_quote!(
			static #ident: &[::mozjs::jsapi::JSPropertySpec] = &[
				::mozjs::jsapi::JSPropertySpec::ZERO,
			];
		)
	} else {
		parse_quote!(
			static #ident: &[::mozjs::jsapi::JSPropertySpec] = &[
				#(#specs),*,
				::mozjs::jsapi::JSPropertySpec::ZERO,
			];
		)
	}
}
