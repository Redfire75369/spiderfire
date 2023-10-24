/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Span};
use syn::{Arm, Block, Data, DeriveInput, Error, Fields, Generics, ItemImpl, parse2, Result};
use syn::spanned::Spanned;

use crate::utils::add_trait_bounds;

pub(super) fn impl_trace(mut input: DeriveInput) -> Result<ItemImpl> {
	add_trait_bounds(&mut input.generics, &parse_quote!(::mozjs::gc::Traceable));
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	let impl_generics: Generics = parse2(quote_spanned!(impl_generics.span() => #impl_generics))?;

	let name = &input.ident;
	let body = impl_body(input.span(), &input.data)?;

	parse2(quote_spanned!(input.span() =>
		#[automatically_derived]
		unsafe impl #impl_generics ::mozjs::gc::Traceable for #name #ty_generics #where_clause {
			unsafe fn trace(&self, __ion_tracer: *mut ::mozjs::jsapi::JSTracer) {
				unsafe #body
			}
		}
	))
}

fn impl_body(span: Span, data: &Data) -> Result<Block> {
	match data {
		Data::Struct(r#struct) => {
			let idents = field_idents(&r#struct.fields);
			parse2(quote_spanned!(span => {
				let Self { #(#idents),* } = self;
				#(::mozjs::gc::Traceable::trace(#idents, __ion_tracer));*
			}))
		}
		Data::Enum(r#enum) => {
			let matches: Vec<Arm> = r#enum
				.variants
				.iter()
				.map(|variant| {
					let ident = &variant.ident;
					let idents = field_idents(&variant.fields);
					parse2(quote_spanned!(variant.span() => Self::#ident(#(#idents),*) => {
						#(::mozjs::gc::Traceable::trace(#idents, __ion_tracer));*
					}))
				})
				.collect::<Result<_>>()?;
			parse2(quote_spanned!(span => {
				match self {
					#(#matches)*
				}
			}))
		}
		Data::Union(_) => Err(Error::new(span, "#[derive(Traceable) is not implemented for union types.")),
	}
}

fn field_idents(fields: &Fields) -> Vec<Ident> {
	let fields = match fields {
		Fields::Named(fields) => &fields.named,
		Fields::Unnamed(fields) => &fields.unnamed,
		Fields::Unit => return Vec::new(),
	};
	fields
		.iter()
		.enumerate()
		.map(|(index, field)| match &field.ident {
			Some(ident) => ident.clone(),
			None => format_ident!("var{}", index),
		})
		.collect()
}
