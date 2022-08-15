/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use quote::ToTokens;
use syn::{Error, Expr, FnArg, LitStr, Pat, PatType, Stmt, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::utils::type_ends_with;

#[derive(Debug)]
pub(crate) enum Parameter {
	Context(PatType),
	Arguments(PatType),
	This(PatType),
	VarArgs(PatType, Box<Expr>),
	Normal(PatType, Box<Expr>),
}

impl Parameter {
	pub(crate) fn from_arg(arg: &FnArg) -> syn::Result<Parameter> {
		if let FnArg::Typed(arg) = arg {
			return if arg.ty == parse_quote!(Context) {
				Ok(Parameter::Context(arg.clone()))
			} else if arg.ty == parse_quote!(&Arguments) {
				Ok(Parameter::Arguments(arg.clone()))
			} else {
				let mut convert = None;
				let mut vararg = false;
				for attr in &arg.attrs {
					if attr == &parse_quote!(#[this]) {
						return Ok(Parameter::This(arg.clone()));
					} else if attr == &parse_quote!(#[varargs]) {
						vararg = true;
					} else if attr.path == parse_quote!(convert) {
						let convert_ty: Expr = attr.parse_args()?;
						convert = Some(convert_ty);
					}
				}

				if vararg {
					Ok(Parameter::VarArgs(arg.clone(), Box::new(convert.unwrap_or(parse_quote!(())))))
				} else {
					Ok(Parameter::Normal(arg.clone(), Box::new(convert.unwrap_or(parse_quote!(())))))
				}
			};
		}

		Err(Error::new(arg.span(), "Received Self"))
	}

	pub(crate) fn into_statement(self, index: &mut usize) -> Stmt {
		let krate = quote!(::ion);
		use Parameter::*;
		match self {
			Context(PatType { pat, ty, .. }) => parse_quote!(let #pat: #ty = cx;),
			Arguments(PatType { pat, ty, .. }) => parse_quote!(let #pat: #ty = args;),
			This(PatType { pat, ty, .. }) => {
				let unwrapped = unwrap_param(parse_quote!(#index), pat.clone(), ty.clone(), parse_quote!(args.this()), parse_quote!(()));
				parse_quote!(let #pat: #ty = #unwrapped?;)
			}
			VarArgs(PatType { pat, ty, .. }, conversion) => {
				let id = *index;
				let unwrapped = unwrap_param(parse_quote!(#id + #index), pat.clone(), ty.clone(), parse_quote!(handle), conversion);
				parse_quote! {
					let #pat: #ty = args.range_handles(#index..(args.len() + 1)).iter().enumerate().map(|(index, handle)| #unwrapped)
						.collect::<#krate::Result<_>>()?;
				}
			}
			Normal(PatType { pat, ty, .. }, conversion) => {
				let unwrapped = unwrap_param(
					parse_quote!(#index),
					pat.clone(),
					ty.clone(),
					parse_quote!(args.handle_or_undefined(#index)),
					conversion,
				);
				*index += 1;
				parse_quote!(let #pat: #ty = #unwrapped?;)
			}
		}
	}

	pub(crate) fn into_class_statement(self, index: &mut usize) -> Stmt {
		use Parameter::*;

		let krate = quote!(::ion);
		match self {
			This(PatType { pat, ty: ref_ty, .. }) => {
				let ty = if let Type::Reference(ty) = &*ref_ty {
					ty.elem.clone()
				} else {
					ref_ty.clone()
				};
				parse_quote!(
					let #pat: #ref_ty = {
						use #krate::ClassInitialiser;
						<#ty>::get_private(cx, args.this().to_object(), Some(args))
					};
				)
			}
			param => param.into_statement(index),
		}
	}
}

pub(crate) fn unwrap_param(index: Box<Expr>, pat: Box<Pat>, ty: Box<Type>, handle: Box<Expr>, conversion: Box<Expr>) -> Expr {
	let krate = quote!(::ion);
	let error_msg = format!(
		"Failed to convert argument {} at index {}, to {}",
		pat.to_token_stream(),
		index.to_token_stream(),
		ty.to_token_stream()
	);
	let error = LitStr::new(&error_msg, pat.span());
	parse_quote! {
		if let Some(value) = unsafe { #krate::types::values::from_value(cx, #handle.get(), #conversion) } {
			Ok(value)
		} else {
			Err(#krate::Error::new(#error))
		}
	}
}

pub(crate) fn extract_params(params: &Punctuated<FnArg, Token![,]>, class: bool) -> syn::Result<(Vec<Stmt>, usize, Option<Ident>)> {
	let mut index = 0;

	let mut nargs = 0;
	let mut this: Option<Ident> = None;

	let statements: Vec<_> = params
		.iter()
		.map(|arg| {
			let param = Parameter::from_arg(arg)?;
			match &param {
				Parameter::Normal(ty, _) => {
					if let Type::Path(ty) = &*ty.ty {
						if !type_ends_with(ty, "Option") {
							nargs += 1;
						}
					}
				}
				Parameter::This(pat) => {
					if let Pat::Ident(ident) = &*pat.pat {
						this = Some(ident.ident.clone());
					}
				}
				_ => {}
			}

			if !class {
				Ok(param.into_statement(&mut index))
			} else {
				Ok(param.into_class_statement(&mut index))
			}
		})
		.collect::<syn::Result<_>>()?;

	Ok((statements, nargs, this))
}
