/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Span};
use syn::{Arm, Block, Data, DeriveInput, Error, Fields, Generics, ItemImpl, parse2, Result};
use syn::spanned::Spanned;

use crate::attribute::ParseAttribute;
use crate::attribute::trace::TraceAttribute;
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
				#[allow(unused_unsafe)]
				unsafe #body
			}
		}
	))
}

fn impl_body(span: Span, data: &Data) -> Result<Box<Block>> {
	match data {
		Data::Struct(r#struct) => {
			let (idents, skip) = field_idents(&r#struct.fields)?;
			let traced = idents
				.iter()
				.enumerate()
				.filter_map(|(index, ident)| (!skip.contains(&index)).then_some(ident));
			parse2(quote_spanned!(span => {
				let Self { #(#idents,)* } = self;
				#(::mozjs::gc::Traceable::trace(#traced, __ion_tracer));*
			}))
		}
		Data::Enum(r#enum) => {
			let matches: Vec<Arm> = r#enum
				.variants
				.iter()
				.map(|variant| {
					let ident = &variant.ident;
					let (idents, skip) = field_idents(&variant.fields)?;
					let traced = idents
						.iter()
						.enumerate()
						.filter_map(|(index, ident)| (!skip.contains(&index)).then_some(ident));
					match &variant.fields {
						Fields::Named(_) => parse2(quote_spanned!(variant.span() => Self::#ident { #(#idents,)* } => {
							#(::mozjs::gc::Traceable::trace(#traced, __ion_tracer));*
						})),
						Fields::Unnamed(_) => parse2(quote_spanned!(variant.span() => Self::#ident(#(#idents,)* ) => {
							#(::mozjs::gc::Traceable::trace(#traced, __ion_tracer));*
						})),
						Fields::Unit => parse2(quote_spanned!(variant.span() => Self::#ident => {})),
					}
				})
				.collect::<Result<_>>()?;
			parse2(quote_spanned!(span => {
				match self {
					#(#matches)*
				}
			}))
		}
		Data::Union(_) => Err(Error::new(
			span,
			"#[derive(Traceable)] is not implemented for union types.",
		)),
	}
}

fn field_idents(fields: &Fields) -> Result<(Vec<Ident>, Vec<usize>)> {
	let fields = match fields {
		Fields::Named(fields) => &fields.named,
		Fields::Unnamed(fields) => &fields.unnamed,
		Fields::Unit => return Ok((Vec::new(), Vec::new())),
	};
	let mut skip = Vec::new();
	let idents = fields
		.iter()
		.enumerate()
		.map(|(index, field)| {
			let attribute = TraceAttribute::from_attributes("trace", &field.attrs)?;
			if attribute.no_trace {
				skip.push(index);
			}
			match &field.ident {
				Some(ident) => Ok(ident.clone()),
				None => Ok(format_ident!("var{}", index)),
			}
		})
		.collect::<Result<_>>()?;
	Ok((idents, skip))
}
