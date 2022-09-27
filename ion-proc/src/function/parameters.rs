/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use quote::ToTokens;
use syn::{Error, Expr, FnArg, LitStr, Pat, PathArguments, PatType, Result, Stmt, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::function::attribute::ParameterAttribute;
use crate::utils::type_ends_with;

#[derive(Debug)]
pub(crate) enum Parameter {
	Context(PatType),
	Arguments(PatType),
	This(PatType),
	VarArgs(PatType, Box<Expr>),
	Normal(PatType, Box<Expr>),
}

pub(crate) struct Parameters {
	pub parameters: Vec<Parameter>,
	pub idents: Vec<Ident>,
	pub nargs: (usize, usize),
	pub this: Option<Ident>,
}

impl Parameter {
	pub(crate) fn from_arg(arg: &FnArg, ident: Option<&Ident>, is_class: bool) -> Result<Parameter> {
		match arg {
			FnArg::Typed(ty) => {
				if ty.ty == parse_quote!(Context) {
					Ok(Parameter::Context(ty.clone()))
				} else if ty.ty == parse_quote!(&Arguments) {
					Ok(Parameter::Arguments(ty.clone()))
				} else if ty.pat == parse_quote!(self) && is_class {
					if let Type::Path(path) = &*ty.ty {
						if type_ends_with(path, "Box") {
							let arg = parse_quote!(self: ::std::boxed::Box<#ident>);
							if let FnArg::Typed(ty) = arg {
								return Ok(Parameter::This(ty));
							}
						}
						Err(Error::new(arg.span(), "Invalid type for self"))
					} else {
						Ok(Parameter::This(ty.clone()))
					}
				} else {
					let mut conversion = None;
					let mut vararg = false;

					for attr in &ty.attrs {
						if attr.path.is_ident("ion") {
							let args: Punctuated<ParameterAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

							for arg in args {
								match arg {
									ParameterAttribute::This(_) => return Ok(Parameter::This(ty.clone())),
									ParameterAttribute::VarArgs(_) => {
										vararg = true;
									}
									ParameterAttribute::Convert { conversion: conversion_expr, .. } => {
										conversion = Some(conversion_expr);
									}
								}
							}
						}
					}

					let conversion = Box::new(conversion.unwrap_or_else(|| parse_quote!(())));

					if vararg {
						Ok(Parameter::VarArgs(ty.clone(), conversion))
					} else {
						Ok(Parameter::Normal(ty.clone(), conversion))
					}
				}
			}
			FnArg::Receiver(recv) => {
				if !is_class || recv.reference.is_none() {
					return Err(Error::new(arg.span(), "Received Self"));
				}
				let lifetime = recv.reference.as_ref().and_then(|(_, l)| l.as_ref());
				let mutability = recv.mutability;
				let this = <Token![self]>::default().into();
				let ident = ident.unwrap_or(&this);
				let arg = parse_quote!(self: &#lifetime #mutability #ident);
				if let FnArg::Typed(ty) = arg {
					Ok(Parameter::This(ty))
				} else {
					Err(Error::new(arg.span(), ""))
				}
			}
		}
	}

	pub(crate) fn to_statement(&self, index: &mut usize) -> Stmt {
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
				let unwrapped = unwrap_param(
					parse_quote!(#index + index),
					pat.clone(),
					ty.clone(),
					parse_quote!(handle),
					conversion.clone(),
				);
				parse_quote! {
					let #pat: #ty = args.range_handles(#index..=args.len()).iter().enumerate().map(|(index, handle)| #unwrapped)
						.collect::<#krate::Result<_>>()?;
				}
			}
			Normal(PatType { pat, ty, .. }, conversion) => {
				let unwrapped = unwrap_param(
					parse_quote!(#index),
					pat.clone(),
					ty.clone(),
					parse_quote!(args.handle_or_undefined(#index)),
					conversion.clone(),
				);
				*index += 1;
				parse_quote!(let #pat: #ty = #unwrapped?;)
			}
		}
	}

	pub(crate) fn to_class_statement(&self, index: &mut usize) -> Result<Stmt> {
		use Parameter::*;

		let krate = quote!(::ion);
		match self {
			This(pat_ty) => {
				let PatType { pat, ty: ref_ty, .. } = pat_ty;
				let pat = if **pat == parse_quote!(self) {
					parse_quote!(self_)
				} else {
					*pat.clone()
				};
				match &**ref_ty {
					Type::Reference(ty) => {
						let ty = ty.elem.clone();
						Ok(parse_quote!(
							let #pat: #ref_ty = <#ty as #krate::ClassInitialiser>::get_private(cx, #krate::Object::from(args.this().to_object()), ::std::option::Option::Some(args))?;
						))
					}
					Type::Path(ty) if type_ends_with(ty, "Box") => {
						let ty = ty.clone();
						if let PathArguments::AngleBracketed(args) = &ty.path.segments.last().as_ref().unwrap().arguments {
							let ty = args.args.first().unwrap();
							Ok(parse_quote!(
								let #pat: #ref_ty = <#ty as #krate::ClassInitialiser>::take_private(cx, #krate::Object::from(args.this().to_object()), ::std::option::Option::Some(args))?;
							))
						} else {
							unreachable!()
						}
					}
					_ => Err(Error::new(pat_ty.span(), "Found Invalid This")),
				}
			}
			param => Ok(param.to_statement(index)),
		}
	}
}

impl Parameters {
	pub(crate) fn parse(parameters: &Punctuated<FnArg, Token![,]>, ident: Option<&Ident>, is_class: bool) -> Result<Parameters> {
		let mut nargs = (0, 0);
		let mut this: Option<Ident> = None;
		let mut idents = Vec::new();

		let parameters: Vec<_> = parameters
			.iter()
			.map(|arg| {
				let param = Parameter::from_arg(arg, ident, is_class)?;
				let ident = match &param {
					Parameter::Normal(ty, _) => {
						if let Type::Path(ty) = &*ty.ty {
							if !type_ends_with(ty, "Option") {
								nargs.0 += 1;
							} else {
								nargs.1 += 1;
							}
						}
						get_ident(&*ty.pat)
					}
					Parameter::This(pat) => {
						if let Pat::Ident(ident) = &*pat.pat {
							this = Some(ident.ident.clone());
							if ident.ident != "self" {
								get_ident(&*pat.pat)
							} else {
								None
							}
						} else {
							None
						}
					}
					Parameter::Context(ty) | Parameter::Arguments(ty) | Parameter::VarArgs(ty, _) => get_ident(&*ty.pat),
				};

				if let Some(ident) = ident {
					idents.push(ident);
				}
				Ok(param)
			})
			.collect::<Result<_>>()?;

		Ok(Parameters { parameters, idents, nargs, this })
	}

	pub(crate) fn to_statements(&self, is_class: bool) -> Result<Vec<Stmt>> {
		let mut index = 0;
		self.parameters
			.iter()
			.map(|parameter| {
				if !is_class {
					Ok(parameter.to_statement(&mut index))
				} else {
					parameter.to_class_statement(&mut index)
				}
			})
			.collect()
	}

	pub(crate) fn to_args(&self) -> Vec<FnArg> {
		let mut index = None;
		let mut args = self
			.parameters
			.iter()
			.enumerate()
			.map(|(i, parameter)| match parameter {
				Parameter::Context(ty) | Parameter::Arguments(ty) | Parameter::Normal(ty, _) | Parameter::VarArgs(ty, _) => {
					let mut ty = ty.clone();
					ty.attrs.clear();
					FnArg::Typed(ty)
				}
				Parameter::This(ty) => {
					let mut ty = ty.clone();
					ty.attrs.clear();
					if ty.pat == parse_quote!(self) {
						index = Some(i);
					}
					FnArg::Typed(ty)
				}
			})
			.collect::<Vec<_>>();
		if let Some(index) = index {
			let this = args.remove(index);
			args.insert(0, this);
		}
		args
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
		if let ::std::option::Option::Some(value) = #krate::types::values::from_value(cx, #handle.get(), #conversion) {
			::std::result::Result::Ok(value)
		} else {
			::std::result::Result::Err(#krate::Error::new(#error, ::std::option::Option::Some(#krate::ErrorKind::Type)))
		}
	}
}

pub(crate) fn get_ident(pat: &Pat) -> Option<Ident> {
	if let Pat::Ident(ident) = pat {
		Some(ident.ident.clone())
	} else {
		None
	}
}
