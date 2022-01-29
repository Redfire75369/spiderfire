/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{Error, Expr, FnArg, Pat, PatType, Stmt, Type};
use syn::spanned::Spanned;

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
			return if arg.ty == parse_quote!(IonContext) {
				Ok(Parameter::Context(arg.clone()))
			} else if arg.ty == parse_quote!(&Arguments) {
				Ok(Parameter::Arguments(arg.clone()))
			} else {
				let mut convert = None;
				let mut vararg = false;
				for attr in arg.attrs.clone() {
					if attr == parse_quote!(#[this]) {
						return Ok(Parameter::This(arg.clone()));
					} else if attr == parse_quote!(#[varargs]) {
						vararg = true;
					} else if attr.path == parse_quote!(convert) {
						let convert_ty: Expr = attr.parse_args()?;
						convert = Some(convert_ty);
					}
				}

				if vararg {
					Ok(Parameter::VarArgs(arg.clone(), Box::new(convert.unwrap_or_else(|| parse_quote!(())))))
				} else {
					Ok(Parameter::Normal(arg.clone(), Box::new(convert.unwrap_or_else(|| parse_quote!(())))))
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
						.collect::<#krate::IonResult<_>>()?;
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

	pub(crate) fn is_normal(&self) -> bool {
		matches!(self, Parameter::Normal(..))
	}
}

fn unwrap_param(index: Box<Expr>, pat: Box<Pat>, ty: Box<Type>, handle: Box<Expr>, conversion: Box<Expr>) -> Expr {
	let krate = quote!(::ion);
	parse_quote! {
		if let Some(value) = unsafe { #krate::types::values::from_value(cx, #handle.get(), #conversion) } {
			Ok(value)
		} else {
			Err(#krate::error::IonError::TypeError(::std::format!(
				"Failed to convert argument {} at index {}, to {}",
				::std::stringify!(#pat),
				#index,
				::std::stringify!(#ty)
			)))
		}
	}
}
