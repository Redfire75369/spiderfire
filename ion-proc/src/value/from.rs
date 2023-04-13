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

use crate::utils::{add_trait_bounds, format_type, type_ends_with};
use crate::value::attribute::{DataAttribute, DefaultValue, FieldAttribute, Tag, VariantAttribute};

pub(crate) fn impl_from_value(mut input: DeriveInput) -> Result<ItemImpl> {
	let krate = quote!(::ion);

	add_trait_bounds(&mut input.generics, &parse2(quote!(#krate::conversions::FromValue)).unwrap());
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

	let mut tag = Tag::default();
	let mut inherit = false;
	let mut repr = None;
	for attr in &input.attrs {
		if attr.path().is_ident("ion") {
			let args: Punctuated<DataAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

			for arg in args {
				match arg {
					DataAttribute::Tag(data_tag) => {
						tag = data_tag;
					}
					DataAttribute::Inherit(_) => {
						inherit = true;
					}
				}
			}
		} else if attr.path().is_ident("repr") {
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

	let (body, requires_object) = impl_body(&input.data, name, input.span(), tag, inherit, repr)?;

	let object = requires_object.then(|| quote_spanned!(input.span() => let object = #krate::Object::from_value(cx, value, true, ())?;));

	parse2(quote_spanned!(input.span() =>
		#[automatically_derived]
		impl #impl_generics #krate::conversions::FromValue<'cx> for #name #ty_generics #where_clause {
			type Config = ();

			unsafe fn from_value<'v>(cx: &'cx #krate::Context, value: &#krate::Value<'v>, strict: bool, _: ())
				-> #krate::Result<Self> where 'cx: 'v
			{
				#object
				#body
			}
		}
	))
}

fn impl_body(data: &Data, ident: &Ident, span: Span, tag: Tag, inherit: bool, repr: Option<Ident>) -> Result<(Block, bool)> {
	match data {
		Data::Struct(data) => match &data.fields {
			Fields::Named(fields) => {
				let (requirement, idents, declarations, requires_object) = map_fields(&fields.named, None, tag, inherit)?;
				parse2(quote_spanned!(span => {
					#requirement
					#(#declarations)*
					::std::result::Result::Ok(Self { #(#idents, )* })
				}))
				.map(|b| (b, requires_object))
			}
			Fields::Unnamed(fields) => {
				let (requirement, idents, declarations, requires_object) = map_fields(&fields.unnamed, None, tag, inherit)?;
				parse2(quote_spanned!(span => {
					#requirement
					#(#declarations)*
					::std::result::Result::Ok(Self(#(#idents, )*))
				}))
				.map(|block| (block, requires_object))
			}
			Fields::Unit => parse2(quote_spanned!(span => { ::std::result::Result::Ok(Self) })).map(|block| (block, false)),
		},
		Data::Enum(data) => {
			let krate = quote!(::ion);
			let unit = data.variants.iter().all(|variant| matches!(variant.fields, Fields::Unit));

			let variants: Vec<(Block, _)> = data
				.variants
				.iter()
				.map(|variant| {
					let variant_ident = &variant.ident;
					let variant_string = variant_ident.to_string();

					let mut tag = tag.clone();
					let mut inherit = inherit;

					for attr in &variant.attrs {
						if attr.path().is_ident("ion") {
							let args: Punctuated<VariantAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

							for arg in args {
								match arg {
									VariantAttribute::Tag(variant_tag) => {
										tag = variant_tag;
									}
									VariantAttribute::Inherit(_) => {
										inherit = true;
									}
								}
							}
						}
					}

					let handle_result = quote!(if let ::std::result::Result::Ok(success) = variant {
						return ::std::result::Result::Ok(success);
					});
					match &variant.fields {
						Fields::Named(fields) => {
							let (requirement, idents, declarations, requires_object) = map_fields(&fields.named, Some(variant_string), tag, inherit)?;
							parse2(quote_spanned!(variant.span() => {
								let variant: #krate::Result<Self> = (|| {
									#requirement
									#(#declarations)*
									::std::result::Result::Ok(Self::#variant_ident { #(#idents, )* })
								})();
								#handle_result
							}))
							.map(|block| (block, requires_object))
						}
						Fields::Unnamed(fields) => {
							let (requirement, idents, declarations, requires_object) =
								map_fields(&fields.unnamed, Some(variant_string), tag, inherit)?;
							parse2(quote_spanned!(variant.span() => {
								let variant: #krate::Result<Self> = (|| {
									#requirement
									#(#declarations)*
									::std::result::Result::Ok(Self::#variant_ident(#(#idents, )*))
								})();
								#handle_result
							}))
							.map(|block| (block, requires_object))
						}
						Fields::Unit => {
							if let Some((_, discriminant)) = &variant.discriminant {
								if unit && repr.is_some() {
									return parse2(quote_spanned!(
										variant.fields.span() => {
											if discriminant == #discriminant {
												return ::std::result::Result::Ok(Self::#variant_ident);
											}
										}
									))
									.map(|block| (block, false));
								}
							}
							parse2(quote!({return ::std::result::Result::Ok(Self::#variant_ident);})).map(|block| (block, false))
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
					if_unit = Some(
						quote_spanned!(repr.span() => let discriminant: #repr = #krate::conversions::FromValue::from_value(cx, value, true, #krate::conversions::ConversionBehavior::EnforceRange)?;),
					);
				}
			}

			parse2(quote_spanned!(span => {
				#if_unit
				#(#variants)*

				::std::result::Result::Err(#krate::Error::new(#error, #krate::ErrorKind::Type))
			}))
			.map(|b| (b, requires_object))
		}
		Data::Union(_) => Err(Error::new(span, "#[derive(FromValue)] is not implemented for union types")),
	}
}

fn map_fields(
	fields: &Punctuated<Field, Token![,]>, variant: Option<String>, tag: Tag, inherit: bool,
) -> Result<(TokenStream, Vec<Ident>, Vec<TokenStream>, bool)> {
	let krate = quote!(::ion);
	let mut is_tagged = None;

	let requirement = match tag {
		Tag::Untagged(_) => quote!(),
		Tag::External(kw) => {
			is_tagged = Some(kw);
			if let Some(variant) = variant {
				let error = format!("Expected Object at External Tag {}", variant);
				quote_spanned!(kw.span() =>
					let object: #krate::Object = object.get_as(cx, #variant, true, ())
						.ok_or_else(|| #krate::Error::new(#error, #krate::ErrorKind::Type))?;
				)
			} else {
				return Err(Error::new(kw.span(), "Cannot have Tag for Struct"));
			}
		}
		Tag::Internal { kw, key, .. } => {
			is_tagged = Some(kw);
			if let Some(variant) = variant {
				let missing_error = format!("Expected Internal Tag key {}", key.value());
				let error = format!("Expected Internal Tag {} at key {}", variant, key.value());
				quote_spanned!(kw.span() =>
					let key: ::std::string::String = object.get_as(cx, #key, true, ()).ok_or_else(|| #krate::Error::new(#missing_error, #krate::ErrorKind::Type))?;
					if key != #variant {
						return Err(#krate::Error::new(#error, #krate::ErrorKind::Type));
					}
				)
			} else {
				return Err(Error::new(kw.span(), "Cannot have Tag for Struct"));
			}
		}
	};
	let mut requires_object = is_tagged.is_some();

	let vec: Vec<_> = fields
		.iter()
		.enumerate()
		.map(|(index, field)| {
			let (ident, key) = if let Some(ref ident) = field.ident {
				(ident.clone(), ident.to_string().to_case(Case::Camel))
			} else {
				let ident = Ident::new(&format!("var{}", index), field.span());
				(ident, index.to_string())
			};

			let ty = &field.ty;
			let attrs = &field.attrs;

			let mut optional = false;
			if let Type::Path(ty) = ty {
				if type_ends_with(ty, "Option") {
					optional = true;
				}
			}

			let mut convert = None;
			let mut strict = false;
			let mut default = None;
			let mut parser = None;
			let mut inherit = inherit;

			for attr in attrs {
				if attr.path().is_ident("ion") {
					let args: Punctuated<FieldAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;
					for arg in args {
						use FieldAttribute as FA;
						match arg {
							FA::Inherit(_) => {
								inherit = true;
							}
							FA::Convert { expr, .. } => {
								convert = Some(expr);
							}
							FA::Strict(_) => {
								strict = true;
							}
							FA::Default { def, .. } => {
								default = Some(def);
							}
							FA::Parser { expr, .. } => {
								parser = Some(expr);
							}
						}
					}
				}
			}

			let convert = convert.unwrap_or_else(|| parse_quote!(()));

			let base = if inherit {
				if is_tagged.is_some() {
					return Err(Error::new(field.span(), "Inherited Field cannot be parsed from a Tagged Enum"));
				}
				quote_spanned!(field.span() => let #ident: #ty = <#ty as #krate::conversions::FromValue>::from_value(cx, value, #strict || strict, #convert))
			} else if let Some(ref parser) = parser {
				requires_object = true;
				let error = format!("Expected Value at Key {}", key);
				quote_spanned!(field.span() => let #ident: #ty = object.get(cx, #key).ok_or_else(|| #krate::Error::new(#error, #krate::ErrorKind::Type)).and_then(#parser))
			} else {
				requires_object = true;
				let error = format!("Expected Value at key {} of Type {}", key, format_type(ty));
				let ok_or_else = quote!(.ok_or_else(|| #krate::Error::new(#error, #krate::ErrorKind::Type)));
				quote_spanned!(field.span() => let #ident: #ty = object.get_as(cx, #key, #strict || strict, #convert)#ok_or_else)
			};

			let stmt = if optional {
				quote_spanned!(field.span() => #base.ok();)
			} else {
				match default {
					Some(Some(DefaultValue::Expr(expr))) => {
						if inherit {
							return Err(Error::new(
								field.span(),
								"Cannot have Default Expression with Inherited Field. Use a Closure with One Argument Instead",
							));
						} else {
							quote_spanned!(field.span() => #base.unwrap_or_else(|_| #expr);)
						}
					}
					Some(Some(DefaultValue::Closure(closure))) => quote_spanned!(field.span() => #base.unwrap_or_else(#closure);),
					Some(Some(DefaultValue::Literal(lit))) => quote_spanned!(field.span() => #base.unwrap_or(#lit);),
					Some(None) => quote_spanned!(field.span() => #base.unwrap_or_default();),
					None => quote_spanned!(field.span() => #base?;),
				}
			};

			Ok((ident, stmt))
		})
		.collect::<Result<_>>()?;

	let (idents, declarations) = vec.into_iter().unzip();
	Ok((requirement, idents, declarations, requires_object))
}
