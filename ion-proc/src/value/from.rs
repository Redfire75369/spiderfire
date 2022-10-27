/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use syn::{Block, Data, DeriveInput, Error, Field, Fields, GenericParam, Generics, ItemImpl, parse2, Result, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::utils::{add_trait_bounds, format_type, type_ends_with};
use crate::value::attribute::{DefaultValue, FromValueAttribute};

pub(crate) fn impl_from_value(mut input: DeriveInput) -> Result<ItemImpl> {
	let krate = quote!(::ion);

	let span = input.span();
	let name = &input.ident;

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

	let (body, requires_obj) = impl_body(&input.data, name, span)?;

	let object = requires_obj.then(|| quote!(let object = #krate::Object::from_value(cx, val, true, ())?;));

	parse2(quote!(
		#[allow(unused_qualifications)]
		impl #impl_generics #krate::conversions::FromValue<'cx> for #name #ty_generics #where_clause {
			type Config = ();

			unsafe fn from_value<'v>(cx: &'cx #krate::Context, val: &#krate::Value<'v>, strict: bool, _: ())
				-> #krate::Result<Self> where 'cx: 'v
			{
				#object
				#body
			}
		}
	))
}

fn impl_body(data: &Data, ident: &Ident, span: Span) -> Result<(Block, bool)> {
	match data {
		Data::Struct(data) => match &data.fields {
			Fields::Named(fields) => {
				let (idents, declarations, requires_obj) = map_fields(&fields.named, false)?;
				parse2(quote!({
					#(#declarations)*
					::std::result::Result::Ok(Self { #(#idents, )* })
				}))
				.map(|b| (b, requires_obj))
			}
			Fields::Unnamed(fields) => {
				let (idents, declarations, requires_obj) = map_fields(&fields.unnamed, false)?;
				parse2(quote!({
					#(#declarations)*
					::std::result::Result::Ok(Self(#(#idents, )*))
				}))
				.map(|block| (block, requires_obj))
			}
			Fields::Unit => parse2(quote!({ ::std::result::Result::Ok(Self) })).map(|block| (block, false)),
		},
		Data::Enum(data) => {
			let krate = quote!(::ion);
			let variants: Vec<(Block, _)> = data
				.variants
				.iter()
				.map(|variant| {
					let variant_ident = &variant.ident;

					let mut inherit = true;

					for attr in &variant.attrs {
						if attr.path.is_ident("ion") {
							let args: Punctuated<FromValueAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

							for arg in args {
								if let FromValueAttribute::Inherit(_) = arg {
									inherit = true;
								}
							}
						}
					}

					let handle_result = quote!(if let ::std::result::Result::Ok(success) = variant {
						return ::std::result::Result::Ok(success);
					});
					match &variant.fields {
						Fields::Named(fields) => {
							let (idents, declarations, requires_obj) = map_fields(&fields.named, inherit)?;
							parse2(quote_spanned!(variant.span() => {
								let variant: #krate::Result<Self> = (|| {
									#(#declarations)*
									::std::result::Result::Ok(Self::#variant_ident { #(#idents, )* })
								})();
								#handle_result
							}))
							.map(|block| (block, requires_obj))
						}
						Fields::Unnamed(fields) => {
							let (idents, declarations, requires_obj) = map_fields(&fields.unnamed, inherit)?;
							parse2(quote_spanned!(variant.span() => {
								let variant: #krate::Result<Self> = (|| {
									#(#declarations)*
									::std::result::Result::Ok(Self::#variant_ident(#(#idents, )*))
								})();
								#handle_result
							}))
							.map(|block| (block, requires_obj))
						}
						Fields::Unit => parse2(quote!({return ::std::result::Result::Ok(Self::#variant_ident);})).map(|block| (block, false)),
					}
				})
				.collect::<Result<_>>()?;
			let (variants, requires_obj): (Vec<_>, Vec<_>) = variants.into_iter().unzip();
			let requires_obj = requires_obj.into_iter().any(|b| b);

			let error = format!("Value does not match any of the enum {}", ident);

			parse2(quote!({
				#(#variants)*

				::std::result::Result::Err(#krate::Error::new(#error, #krate::ErrorKind::Type))
			}))
			.map(|b| (b, requires_obj))
		}
		Data::Union(_) => Err(Error::new(span, "#[derive(FromValue)] is not implemented for union types")),
	}
}

fn map_fields(fields: &Punctuated<Field, Token![,]>, inherit: bool) -> Result<(Vec<Ident>, Vec<TokenStream>, bool)> {
	let krate = quote!(::ion);
	let mut requires_obj = false;
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
				if attr.path.is_ident("ion") {
					let args: Punctuated<FromValueAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;
					for arg in args {
						use FromValueAttribute::*;
						match arg {
							Inherit(_) => {
								inherit = true;
							}
							Optional(_) => {
								optional = true;
							}
							Convert { expr, .. } => {
								convert = Some(expr);
							}
							Strict(_) => {
								strict = true;
							}
							Default { def, .. } => {
								default = Some(def);
							}
							Parser { expr, .. } => {
								parser = Some(expr);
							}
						}
					}
				}
			}

			let convert = convert.unwrap_or_else(|| parse_quote!(()));

			let base = if inherit {
				quote_spanned!(field.span() => let #ident: #ty = <#ty as #krate::conversions::FromValue>::from_value(cx, val, #strict || strict, #convert))
			} else if let Some(ref parser) = parser {
				requires_obj = true;
				quote_spanned!(field.span() => let #ident: #ty = object.get(cx, #key).and_then(#parser))
			} else {
				requires_obj = true;
				quote_spanned!(field.span() => let #ident: #ty = object.get_as(cx, #key, #strict || strict, #convert))
			};

			let stmt = if optional {
				quote_spanned!(field.span() => #base;)
			} else {
				match default {
					Some(Some(DefaultValue::Expr(expr))) => {
						if inherit {
							return Err(Error::new(
								field.span(),
								"Cannot have Default Expression with Inherited Field. Use a Closure with One Argument Instead",
							));
						} else {
							quote_spanned!(field.span() => #base.unwrap_or_else(|| #expr);)
						}
					}
					Some(Some(DefaultValue::Closure(closure))) => quote_spanned!(field.span() => #base.unwrap_or_else(#closure);),
					Some(Some(DefaultValue::Literal(lit))) => quote_spanned!(field.span() => #base.unwrap_or(#lit);),
					Some(None) => quote_spanned!(field.span() => #base.unwrap_or_default();),
					None => {
						if inherit {
							quote_spanned!(field.span() => #base?;)
						} else if parser.is_some() {
							let error = format!("Failed to obtain or parse {} to type {}", key, format_type(ty));
							quote_spanned!(field.span() => #base.ok_or_else(|| #krate::Error::new(#error, #krate::ErrorKind::Type))?;)
						} else {
							let missing_error = format!("Failed to obtain {} of type {}", key, format_type(ty));
							quote_spanned!(field.span() => #base.ok_or_else(|| #krate::Error::new(#missing_error, #krate::ErrorKind::Type))?;)
						}
					}
				}
			};

			Ok((ident, stmt))
		})
		.collect::<Result<_>>()?;

	let (idents, declarations) = vec.into_iter().unzip();
	Ok((idents, declarations, requires_obj))
}
