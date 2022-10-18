/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Block, Data, DeriveInput, Error, Field, Fields, ItemImpl, LitStr, parse2, Result, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::jsval::attribute::{DefaultValue, FromJSValAttribute};
use crate::utils::{add_trait_bounds, type_ends_with};

pub(crate) fn impl_from_jsval(input: DeriveInput) -> Result<ItemImpl> {
	let span = input.span();

	let name = &input.ident;
	let krate = quote!(::ion);

	let generics = add_trait_bounds(input.generics, &parse2(quote!(::mozjs::conversions::FromJSValConvertible)).unwrap());
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	let (body, requires_obj) = impl_body(&input.data, name, span)?;

	let body = if requires_obj {
		quote!(
			match #krate::Object::from_jsval(cx, val, ())? {
				::mozjs::conversions::ConversionResult::Success(obj) => #body,
				::mozjs::conversions::ConversionResult::Failure(err) => ::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Failure(err)),
			}
		)
	} else {
		body.to_token_stream()
	};

	parse2(quote!(
		#[allow(unused_qualifications)]
		impl #impl_generics ::mozjs::conversions::FromJSValConvertible for #name #ty_generics #where_clause {
			type Config = ();

			unsafe fn from_jsval(cx: #krate::Context, val: ::mozjs::rust::HandleValue, _: Self::Config)
				-> ::std::result::Result<::mozjs::conversions::ConversionResult<Self>, ()>
			{
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
					::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Success(Self { #(#idents, )* }))
				}))
				.map(|b| (b, requires_obj))
			}
			Fields::Unnamed(fields) => {
				let (idents, declarations, requires_obj) = map_fields(&fields.unnamed, false)?;
				parse2(quote!({
					#(#declarations)*
					::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Success(Self(#(#idents, )*)))
				}))
				.map(|block| (block, requires_obj))
			}
			Fields::Unit => parse2(quote!({
				::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Success(Self))
			}))
			.map(|block| (block, false)),
		},
		Data::Enum(data) => {
			let variants: Vec<(Block, _)> = data
				.variants
				.iter()
				.map(|variant| {
					let variant_ident = &variant.ident;

					let mut inherit = true;

					for attr in &variant.attrs {
						if attr.path.is_ident("ion") {
							let args: Punctuated<FromJSValAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

							for arg in args {
								if let FromJSValAttribute::Inherit(_) = arg {
									inherit = true;
								}
							}
						}
					}

					let handle_result = quote!(match variant {
						::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Success(success)) =>
							return ::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Success(success)),
						::std::result::Result::Err(_) => ::ion::Exception::clear(cx),
						_ => (),
					});
					match &variant.fields {
						Fields::Named(fields) => {
							let (idents, declarations, requires_obj) = map_fields(&fields.named, inherit)?;
							parse2(quote_spanned!(variant.span() => {
								let variant: ::std::result::Result<::mozjs::conversions::ConversionResult<Self>, ()> = (|| {
									#(#declarations)*
									::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Success(Self::#variant_ident { #(#idents, )* }))
								})();
								#handle_result
							}))
							.map(|block| (block, requires_obj))
						}
						Fields::Unnamed(fields) => {
							let (idents, declarations, requires_obj) = map_fields(&fields.unnamed, inherit)?;
							parse2(quote_spanned!(variant.span() => {
								let variant: ::std::result::Result<::mozjs::conversions::ConversionResult<Self>, ()> = (|| {
									#(#declarations)*
									::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Success(Self::#variant_ident(#(#idents, )*)))
								})();
								#handle_result
							}))
							.map(|block| (block, requires_obj))
						}
						Fields::Unit => {
							parse2(quote!({return ::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Success(Self::#variant_ident));}))
								.map(|block| (block, false))
						}
					}
				})
				.collect::<Result<_>>()?;
			let (variants, requires_obj): (Vec<_>, Vec<_>) = variants.into_iter().unzip();
			let requires_obj = requires_obj.into_iter().any(|b| b);

			let error = LitStr::new(&format!("Value does not match any of the enum {}", ident), span);

			parse2(quote!({
				#(#variants)*

				::std::result::Result::Ok(::mozjs::conversions::ConversionResult::Failure(::std::borrow::Cow::Borrowed(#error)))
			}))
			.map(|b| (b, requires_obj))
		}
		Data::Union(_) => Err(Error::new(span, "#[derive(FromJSVal)] is not implemented for union types")),
	}
}

fn map_fields(fields: &Punctuated<Field, Token![,]>, inherit: bool) -> Result<(Vec<Ident>, Vec<TokenStream>, bool)> {
	let conversions = quote!(::mozjs::conversions);
	let mut requires_obj = false;
	let vec: Vec<_> = fields
		.iter()
		.enumerate()
		.map(|(index, field)| {
			let (ident, key) = if let Some(ref ident) = field.ident {
				let key = LitStr::new(&ident.to_string().to_case(Case::Camel), field.span());
				(ident.clone(), key)
			} else {
				let ident = Ident::new(&format!("var{}", index), field.span());
				let key = LitStr::new(&index.to_string(), field.span());
				(ident, key)
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
			let mut default = None;
			let mut parser = None;
			let mut inherit = inherit;

			for attr in attrs {
				if attr.path.is_ident("ion") {
					let args: Punctuated<FromJSValAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;
					for arg in args {
						use FromJSValAttribute::*;
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
				quote_spanned!(field.span() => let #ident: #ty = match <#ty as #conversions::FromJSValConvertible>::from_jsval(cx, val, #convert)? {
					#conversions::ConversionResult::Success(s) => s,
					#conversions::ConversionResult::Failure(e) => return ::std::result::Result::Ok(#conversions::ConversionResult::Failure(e)),
				})
			} else if let Some(parser) = parser {
				requires_obj = true;
				quote_spanned!(field.span() => let #ident: #ty = obj.get(cx, #key).map(#parser))
			} else {
				requires_obj = true;
				quote_spanned!(field.span() => let #ident: #ty = obj.get_as(cx, #key, #convert))
			};

			let stmt = if optional || inherit {
				quote_spanned!(field.span() => #base;)
			} else {
				match default {
					Some(Some(DefaultValue::Expr(expr))) => quote_spanned!(field.span() => #base.unwrap_or_else(|| #expr);),
					Some(Some(DefaultValue::Literal(lit))) => quote_spanned!(field.span() => #base.unwrap_or(#lit);),
					Some(None) => quote_spanned!(field.span() => #base.unwrap_or_default();),
					None => quote_spanned!(field.span() => #base.unwrap();),
				}
			};

			Ok((ident, stmt))
		})
		.collect::<Result<_>>()?;

	let (idents, declarations) = vec.into_iter().unzip();
	Ok((idents, declarations, requires_obj))
}
