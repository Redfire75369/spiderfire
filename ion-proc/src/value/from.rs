/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use syn::{Block, Data, DeriveInput, Error, Field, Fields, GenericParam, Generics, ItemImpl, Meta, parse2, Result, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::attribute::{Optional, ParseAttribute};
use crate::attribute::krate::crate_from_attributes;
use crate::attribute::value::{DataAttribute, DefaultValue, FieldAttribute, Tag, VariantAttribute};
use crate::utils::{add_trait_bounds, format_type, path_ends_with};

pub(crate) fn impl_from_value(mut input: DeriveInput) -> Result<ItemImpl> {
	let ion = &crate_from_attributes(&mut input.attrs);

	add_trait_bounds(&mut input.generics, &parse_quote!(#ion::conversions::FromValue));
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	let mut impl_generics: Generics = parse2(quote_spanned!(impl_generics.span() => #impl_generics))?;

	let has_cx = impl_generics.params.iter().any(|param| {
		if let GenericParam::Lifetime(lt) = param {
			lt.lifetime == parse_quote!('cx)
		} else {
			false
		}
	});
	if !has_cx {
		impl_generics.params.push(parse2(quote!('cx))?);
	}

	let attribute = DataAttribute::from_attributes("ion", &input.attrs)?;
	let DataAttribute { tag, inherit } = attribute;

	let mut repr = None;
	for attr in &input.attrs {
		if attr.path().is_ident("repr") {
			let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
			let allowed_reprs: Vec<Ident> = vec![
				parse_quote!(i8),
				parse_quote!(i16),
				parse_quote!(i32),
				parse_quote!(i64),
				parse_quote!(u8),
				parse_quote!(u16),
				parse_quote!(u32),
				parse_quote!(u64),
			];
			for meta in nested {
				if let Meta::Path(path) = &meta {
					for allowed_repr in &allowed_reprs {
						if path.is_ident(allowed_repr) {
							if repr.is_none() {
								repr = Some(path.get_ident().unwrap().clone());
							} else {
								return Err(Error::new(meta.span(), "Only One Representation Allowed in #[repr]"));
							}
						}
					}
				}
			}
		}
	}

	let name = &input.ident;

	let (body, requires_object) = impl_body(ion, input.span(), &input.data, name, tag, inherit, repr)?;

	let object = if requires_object {
		Some(quote_spanned!(input.span() =>
			let __object = #ion::Object::from_value(cx, value, true, ())?;
		))
	} else {
		None
	};

	parse2(quote_spanned!(input.span() =>
		#[automatically_derived]
		impl #impl_generics #ion::conversions::FromValue<'cx> for #name #ty_generics #where_clause {
			type Config = ();

			fn from_value<'v>(cx: &'cx #ion::Context, value: &#ion::Value<'v>, strict: bool, _: ()) -> #ion::Result<Self> {
				#object
				#body
			}
		}
	))
}

fn impl_body(
	ion: &TokenStream, span: Span, data: &Data, ident: &Ident, tag: Optional<Tag>, inherit: bool, repr: Option<Ident>,
) -> Result<(Box<Block>, bool)> {
	match data {
		Data::Struct(data) => match &data.fields {
			Fields::Named(fields) => {
				let mapped = map_fields(ion, &fields.named, None, tag, inherit)?;
				let (requirement, idents, declarations, requires_object) = mapped;
				parse2(quote_spanned!(span => {
					#requirement
					#(#declarations)*
					::std::result::Result::Ok(Self { #(#idents, )* })
				}))
				.map(|b| (b, requires_object))
			}
			Fields::Unnamed(fields) => {
				let mapped = map_fields(ion, &fields.unnamed, None, tag, inherit)?;
				let (requirement, idents, declarations, requires_object) = mapped;
				parse2(quote_spanned!(span => {
					#requirement
					#(#declarations)*
					::std::result::Result::Ok(Self(#(#idents, )*))
				}))
				.map(|block| (block, requires_object))
			}
			Fields::Unit => {
				parse2(quote_spanned!(span => { ::std::result::Result::Ok(Self) })).map(|block| (block, false))
			}
		},
		Data::Enum(data) => {
			let unit = data.variants.iter().all(|variant| matches!(variant.fields, Fields::Unit));

			let variants: Vec<(Block, _)> = data
				.variants
				.iter()
				.filter_map(|variant| {
					let variant_ident = &variant.ident;
					let variant_string = variant_ident.to_string();

					let old_tag = tag.clone();
					let old_inherit = inherit;

					let attribute = match VariantAttribute::from_attributes("ion", &variant.attrs) {
						Ok(attribute) => attribute,
						Err(e) => return Some(Err(e)),
					};
					let VariantAttribute { tag, inherit, skip } = attribute;
					let tag = Optional(old_tag.0.or(tag.0));
					let inherit = old_inherit || inherit;
					if skip {
						return None;
					}

					let handle_result = quote!(if let ::std::result::Result::Ok(success) = variant {
						return ::std::result::Result::Ok(success);
					});
					match &variant.fields {
						Fields::Named(fields) => {
							let mapped = match map_fields(ion, &fields.named, Some(variant_string), tag, inherit) {
								Ok(mapped) => mapped,
								Err(e) => return Some(Err(e)),
							};
							let (requirement, idents, declarations, requires_object) = mapped;

							Some(
								parse2(quote_spanned!(variant.span() => {
									let variant: #ion::Result<Self> = (|| {
										#requirement
										#(#declarations)*
										::std::result::Result::Ok(Self::#variant_ident { #(#idents, )* })
									})();
									#handle_result
								}))
								.map(|block| (block, requires_object)),
							)
						}
						Fields::Unnamed(fields) => {
							let mapped = match map_fields(ion, &fields.unnamed, Some(variant_string), tag, inherit) {
								Ok(mapped) => mapped,
								Err(e) => return Some(Err(e)),
							};
							let (requirement, idents, declarations, requires_object) = mapped;

							Some(
								parse2(quote_spanned!(variant.span() => {
									let variant: #ion::Result<Self> = (|| {
										#requirement
										#(#declarations)*
										::std::result::Result::Ok(Self::#variant_ident(#(#idents, )*))
									})();
									#handle_result
								}))
								.map(|block| (block, requires_object)),
							)
						}
						Fields::Unit => {
							if let Some((_, discriminant)) = &variant.discriminant {
								if unit && repr.is_some() {
									return Some(
										parse2(quote_spanned!(
											variant.fields.span() => {
												if discriminant == #discriminant {
													return ::std::result::Result::Ok(Self::#variant_ident);
												}
											}
										))
										.map(|block| (block, false)),
									);
								}
							}
							Some(
								parse2(quote!({return ::std::result::Result::Ok(Self::#variant_ident);}))
									.map(|block| (block, false)),
							)
						}
					}
				})
				.collect::<Result<_>>()?;
			let (variants, requires_object): (Vec<_>, Vec<_>) = variants.into_iter().unzip();
			let requires_object = requires_object.into_iter().any(|b| b);

			let error = format!("Value does not match any of the variants of enum {}", ident);

			let mut if_unit = None;

			if unit {
				if let Some(repr) = repr {
					if_unit = Some(quote_spanned!(repr.span() =>
						let discriminant: #repr = #ion::conversions::FromValue::from_value(cx, value, true, #ion::conversions::ConversionBehavior::EnforceRange)?;
					));
				}
			}

			parse2(quote_spanned!(span => {
				#if_unit
				#(#variants)*

				::std::result::Result::Err(#ion::Error::new(#error, #ion::ErrorKind::Type))
			}))
			.map(|b| (b, requires_object))
		}
		Data::Union(_) => Err(Error::new(
			span,
			"#[derive(FromValue)] is not implemented for union types",
		)),
	}
}

fn map_fields(
	ion: &TokenStream, fields: &Punctuated<Field, Token![,]>, variant: Option<String>, tag: Optional<Tag>,
	inherit: bool,
) -> Result<(TokenStream, Vec<Ident>, Vec<TokenStream>, bool)> {
	let mut requires_object = matches!(tag.0, Some(Tag::External | Tag::Internal(_)));

	let requirement = match tag.0 {
		Some(Tag::External) => {
			if let Some(variant) = variant {
				let error = format!("Expected Object at External Tag {}", variant);
				quote!(
					let __object: #ion::Object = __object.get_as(cx, #variant, true, ())?
						.ok_or_else(|_| #ion::Error::new(#error, #ion::ErrorKind::Type))?;
				)
			} else {
				return Err(Error::new(Span::call_site(), "Cannot have Tag for Struct"));
			}
		}
		Some(Tag::Internal(key)) => {
			if let Some(variant) = variant {
				let missing_error = format!("Expected Internal Tag key {}", key.value());
				let error = format!("Expected Internal Tag {} at key {}", variant, key.value());
				quote!(
					let __key: ::std::string::String = __object.get_as(cx, #key, true, ())?
						.ok_or_else(|| #ion::Error::new(#missing_error, #ion::ErrorKind::Type))?;
					if __key != #variant {
						return Err(#ion::Error::new(#error, #ion::ErrorKind::Type));
					}
				)
			} else {
				return Err(Error::new(Span::call_site(), "Cannot have Tag for Struct"));
			}
		}
		_ => quote!(),
	};

	let vec: Vec<_> = fields
		.iter()
		.enumerate()
		.filter_map(|(index, field)| {
			let (ident, mut key) = if let Some(ident) = &field.ident {
				(ident.clone(), ident.to_string().to_case(Case::Camel))
			} else {
				let ident = format_ident!("field{}", index);
				(ident, index.to_string())
			};

			let ty = &field.ty;

			let mut optional = false;
			if let Type::Path(ty) = ty {
				if path_ends_with(&ty.path, "Option") {
					optional = true;
				}
			}

			let old_inherit = inherit;

			let attribute = match FieldAttribute::from_attributes("ion", &field.attrs) {
				Ok(attribute) => attribute,
				Err(e) => return Some(Err(e)),
			};
			let FieldAttribute {
				name,
				inherit,
				skip,
				convert,
				strict,
				default,
				parser,
			} = attribute;
			if let Some(name) = name {
				key = name.value();
			}
			let inherit = old_inherit || inherit;
			if skip {
				return None;
			}

			let convert = convert.unwrap_or_else(|| parse_quote!(()));

			let base = if inherit {
				if requires_object {
					return Some(Err(Error::new(
						field.span(),
						"Inherited Field cannot be parsed from a Tagged Enum",
					)));
				}
				quote_spanned!(field.span() =>
					let #ident: #ty = <#ty as #ion::conversions::FromValue>::from_value(cx, value, #strict || strict, #convert)
				)
			} else if let Some(parser) = &parser {
				requires_object = true;
				let error = format!("Expected Value at Key {}", key);
				quote_spanned!(field.span() => let #ident: #ty = __object.get(cx, #key)?.map(#parser).transpose()?
					.ok_or_else(|| #ion::Error::new(#error, #ion::ErrorKind::Type)))
			} else {
				requires_object = true;
				let error = format!("Expected Value at key {} of Type {}", key, format_type(ty));
				quote_spanned!(field.span() => let #ident: #ty = __object.get_as(cx, #key, #strict || strict, #convert)?
					.ok_or_else(|| #ion::Error::new(#error, #ion::ErrorKind::Type)))
			};

			let stmt = if optional {
				quote_spanned!(field.span() => #base.ok();)
			} else {
				match default.0 {
					Some(DefaultValue::Expr(expr)) => {
						if inherit {
							return Some(Err(Error::new(
								field.span(),
								"Cannot have Default Expression with Inherited Field. Use a Closure with One Argument Instead",
							)));
						} else {
							quote_spanned!(field.span() => #base.unwrap_or_else(|_| #expr);)
						}
					}
					Some(DefaultValue::Closure(closure)) => {
						quote_spanned!(field.span() => #base.unwrap_or_else(#closure);)
					}
					Some(DefaultValue::Literal(lit)) => quote_spanned!(field.span() => #base.unwrap_or(#lit);),
					Some(DefaultValue::Default) => quote_spanned!(field.span() => #base.unwrap_or_default();),
					None => quote_spanned!(field.span() => #base?;),
				}
			};

			Some(Ok((ident, stmt)))
		})
		.collect::<Result<_>>()?;

	let (idents, declarations) = vec.into_iter().unzip();
	Ok((requirement, idents, declarations, requires_object))
}
