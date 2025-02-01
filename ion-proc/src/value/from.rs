/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use syn::spanned::Spanned;
use syn::{parse2, Block, Data, DeriveInput, Error, Fields, Generics, ItemImpl, Result, Type};

use crate::attribute::krate::crate_from_attributes;
use crate::attribute::value::{DataAttribute, DefaultValue, FieldAttribute, Tag, VariantAttribute};
use crate::attribute::{Optional, ParseAttribute};
use crate::utils::{
	add_lifetime_generic, add_trait_bounds, find_repr, format_type, path_ends_with, wrap_in_fields_group,
};

pub(crate) fn impl_from_value(mut input: DeriveInput) -> Result<ItemImpl> {
	let ion = &crate_from_attributes(&mut input.attrs);

	add_trait_bounds(&mut input.generics, &parse_quote!(#ion::conversions::FromValue));
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	let mut impl_generics: Generics = parse2(quote_spanned!(impl_generics.span() => #impl_generics))?;
	add_lifetime_generic(&mut impl_generics, parse_quote!('cx));

	let attribute = DataAttribute::from_attributes("ion", &input.attrs)?;
	let DataAttribute { tag, inherit } = attribute;

	let repr = find_repr(&input.attrs)?;
	let name = &input.ident;

	let (body, requires_object) = impl_body(ion, input.span(), &input.data, name, tag, inherit, repr)?;

	let object = requires_object.then(|| {
		quote_spanned!(input.span() =>
			let __object = #ion::Object::from_value(cx, value, true, ())?;
		)
	});

	parse2(quote_spanned!(input.span() =>
		#[automatically_derived]
		impl #impl_generics #ion::conversions::FromValue<'cx> for #name #ty_generics #where_clause {
			type Config = ();

			fn from_value(cx: &'cx #ion::Context, value: &#ion::Value, strict: bool, _: ()) -> #ion::Result<Self> {
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
			Fields::Named(_) | Fields::Unnamed(_) => {
				let (requirement, idents, declarations, requires_object) =
					map_fields(ion, &data.fields, None, tag, inherit)?;
				let wrapped = wrap_in_fields_group(idents, &data.fields);

				let block = parse2(quote_spanned!(span => {
					#requirement
					#(#declarations)*
					::std::result::Result::Ok(Self #wrapped)
				}))?;
				Ok((block, requires_object))
			}
			Fields::Unit => Ok((parse_quote_spanned!(span => { ::std::result::Result::Ok(Self) }), false)),
		},
		Data::Enum(data) => {
			let mut requires_object = false;
			let mut requires_discriminant = false;

			let variants: Vec<Block> = data
				.variants
				.iter()
				.filter_map(|variant| {
					let variant_ident = &variant.ident;
					let variant_string = variant_ident.to_string();

					let attribute = match VariantAttribute::from_attributes("ion", &variant.attrs) {
						Ok(attribute) => attribute,
						Err(e) => return Some(Err(e)),
					};
					let VariantAttribute { tag: new_tag, inherit: new_inherit, skip } = attribute;
					let tag = Optional(tag.0.clone().or(new_tag.0));
					let inherit = inherit || new_inherit;
					if skip {
						return None;
					}

					let handle_result = quote!(if let ::std::result::Result::Ok(success) = variant {
						return ::std::result::Result::Ok(success);
					});
					match &variant.fields {
						Fields::Named(_) | Fields::Unnamed(_) => {
							let (requirement, idents, declarations, req_object) =
								match map_fields(ion, &variant.fields, Some(variant_string), tag, inherit) {
									Ok(mapped) => mapped,
									Err(e) => return Some(Err(e)),
								};
							let wrapped = wrap_in_fields_group(idents, &variant.fields);

							if req_object {
								requires_object = true;
							}

							Some(parse2(quote_spanned!(variant.span() => {
								let variant: #ion::Result<Self> = (|| {
									#requirement
									#(#declarations)*
									::std::result::Result::Ok(Self::#variant_ident #wrapped)
								})();
								#handle_result
							})))
						}
						Fields::Unit => match &variant.discriminant {
							Some((_, discriminant)) => repr.is_some().then(|| {
								requires_discriminant = true;
								parse2(quote_spanned!(
									variant.fields.span() => {
										if discriminant == #discriminant {
											return ::std::result::Result::Ok(Self::#variant_ident);
										}
									}
								))
							}),
							None => Some(Ok(
								parse_quote!({return ::std::result::Result::Ok(Self::#variant_ident);}),
							)),
						},
					}
				})
				.collect::<Result<_>>()?;

			let error = format!("Value does not match any of the variants of enum {ident}");

			let mut if_unit = None;

			if requires_discriminant {
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
	ion: &TokenStream, fields: &Fields, variant: Option<String>, tag: Optional<Tag>, inherit: bool,
) -> Result<(TokenStream, Vec<Ident>, Vec<TokenStream>, bool)> {
	let mut requires_object = matches!(tag.0, Some(Tag::External | Tag::Internal(_)));

	let requirement = match tag.0 {
		Some(Tag::External) => {
			if let Some(variant) = variant {
				let error = format!("Expected Object at External Tag {variant}");
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
				let error = format!("Expected Internal Tag {variant} at key {}", key.value());
				quote!(
					let __ion_key: ::std::string::String = __object.get_as(cx, #key, true, ())?
						.ok_or_else(|| #ion::Error::new(#missing_error, #ion::ErrorKind::Type))?;
					if __ion_key != #variant {
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
				let ident = format_ident!("_self_{}", index);
				(ident, index.to_string())
			};

			let ty = &field.ty;

			let mut optional = false;
			if let Type::Path(ty) = ty {
				if path_ends_with(&ty.path, "Option") {
					optional = true;
				}
			}

			let attribute = match FieldAttribute::from_attributes("ion", &field.attrs) {
				Ok(attribute) => attribute,
				Err(e) => return Some(Err(e)),
			};
			let FieldAttribute {
				name,
				inherit: new_inherit,
				skip,
				convert,
				strict,
				default,
				parser,
			} = attribute;
			if let Some(name) = name {
				key = name.value();
			}
			let inherit = inherit || new_inherit;
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
				let error = format!("Expected Value at Key {key}");
				quote_spanned!(field.span() => let #ident: #ty = __object.get(cx, #key)?.map(#parser).transpose()?
					.ok_or_else(|| #ion::Error::new(#error, #ion::ErrorKind::Type)))
			} else {
				requires_object = true;
				let error = format!("Expected Value at key {key} of Type {}", format_type(ty));
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
