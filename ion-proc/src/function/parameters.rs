/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::{Error, Expr, FnArg, Lifetime, LitStr, parse2, Pat, PatType, Result, Stmt, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::function::attribute::ParameterAttribute;
use crate::utils::{extract_type_argument, format_pat, format_type, type_ends_with};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ThisKind {
	Ref(Option<Lifetime>, Option<Token![mut]>),
	Box,
	Owned,
}

#[derive(Debug)]
pub(crate) enum Parameter {
	Regular { pat: Box<Pat>, ty: Box<Type>, conversion: Box<Expr> },
	VarArgs { pat: Box<Pat>, ty: Box<Type>, conversion: Box<Expr> },
	This { pat: Box<Pat>, ty: Box<Type>, kind: ThisKind },
	Context(Box<Pat>),
	Arguments(Box<Pat>),
}

pub(crate) struct Parameters {
	pub parameters: Vec<Parameter>,
	pub idents: Vec<Ident>,
	pub nargs: (usize, usize),
	pub this: Option<Ident>,
}

impl Parameter {
	pub(crate) fn from_arg(arg: &FnArg, ty: Option<&Type>, is_class: bool) -> Result<Parameter> {
		match arg {
			FnArg::Typed(pat_ty) => {
				let span = pat_ty.span();
				let PatType { pat, ty, .. } = pat_ty.clone();
				if let Type::Path(ty) = *ty.clone() {
					if type_ends_with(&ty, "Context") {
						return Ok(Parameter::Context(pat));
					} else if type_ends_with(&ty, "Arguments") {
						return Ok(Parameter::Arguments(pat));
					}
				}
				if is_class && pat == parse_quote!(self) {
					parse_this(pat, ty, true, span)
				} else {
					let mut conversion = None;
					let mut vararg = false;

					for attr in &pat_ty.attrs {
						if attr.path.is_ident("ion") {
							let args: Punctuated<ParameterAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

							for arg in args {
								match arg {
									ParameterAttribute::This(_) => return parse_this(pat, ty, is_class, span),
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

					let conversion = conversion.unwrap_or_else(|| parse_quote!(()));

					if vararg {
						Ok(Parameter::VarArgs { pat, ty, conversion })
					} else {
						Ok(Parameter::Regular { pat, ty, conversion })
					}
				}
			}
			FnArg::Receiver(recv) => {
				if !is_class {
					return Err(Error::new(arg.span(), "Can only have self on Class Methods"));
				}
				if recv.reference.is_none() {
					return Err(Error::new(arg.span(), "Invalid type for self"));
				}
				let lifetime = recv.reference.as_ref().and_then(|(_, l)| l.as_ref());
				let mutability = recv.mutability;
				let this = <Token![self]>::default();
				let this = parse2(quote!(#this)).unwrap();
				let ty = ty.unwrap();

				let ty = parse2(quote!(&#lifetime #mutability #ty)).unwrap();
				parse_this(this, ty, true, recv.span())
			}
		}
	}

	pub(crate) fn to_statement(&self, index: &mut usize) -> Stmt {
		let krate = quote!(::ion);
		use Parameter as P;
		match self {
			P::Regular { pat, ty, conversion } => {
				let handle = parse_quote!(args.handle_or_undefined(#index));
				let unwrapped = unwrap_param(Index::Constant(*index), pat, ty, &handle, conversion);
				*index += 1;
				parse_quote!(let #pat: #ty = #unwrapped?;)
			}
			P::VarArgs { pat, ty, conversion } => {
				let id = Index::Expr(parse_quote!(#index + index));
				let handle = parse_quote!(handle);
				let unwrapped = unwrap_param(id, pat, ty, &handle, conversion);
				parse_quote! {
					let #pat: #ty = args.range_handles(#index..=args.len()).iter().enumerate().map(|(index, handle)| #unwrapped)
						.collect::<#krate::Result<_>>()?;
				}
			}
			P::This { pat, ty, .. } => {
				let handle = parse_quote!(args.this());
				let unwrapped = unwrap_param(Index::Constant(*index), pat, ty, &handle, &parse_quote!(()));
				parse_quote!(let #pat: #ty = #unwrapped?;)
			}
			P::Context(pat) => parse_quote!(let #pat: #krate::Context = cx;),
			P::Arguments(pat) => parse_quote!(let #pat: #krate::Arguments = cx;),
		}
	}

	pub(crate) fn to_class_statement(&self, index: &mut usize) -> Result<Stmt> {
		let krate = quote!(::ion);
		match self {
			Parameter::This { pat, ty, kind } => {
				let pat = if **pat == parse_quote!(self) { parse_quote!(self_) } else { pat.clone() };
				match kind {
					ThisKind::Ref(lt, mutability) => Ok(parse2(quote!(
						let #pat: &#lt #mutability #ty = <#ty as #krate::ClassInitialiser>::get_private(cx, #krate::Object::from(args.this().to_object()), ::std::option::Option::Some(args))?;
					))?),
					ThisKind::Box => Ok(parse2(quote!(
						let #pat: ::std::boxed::Box<#ty> = <#ty as #krate::ClassInitialiser>::take_private(cx, #krate::Object::from(args.this().to_object()), ::std::option::Option::Some(args))?;
					))?),
					ThisKind::Owned => unreachable!(),
				}
			}
			param => Ok(param.to_statement(index)),
		}
	}
}

impl Parameters {
	pub(crate) fn parse(parameters: &Punctuated<FnArg, Token![,]>, ty: Option<&Type>, is_class: bool) -> Result<Parameters> {
		let mut nargs = (0, 0);
		let mut this: Option<Ident> = None;
		let mut idents = Vec::new();

		let parameters: Vec<_> = parameters
			.iter()
			.map(|arg| {
				let param = Parameter::from_arg(arg, ty, is_class)?;
				let ident = match &param {
					Parameter::Regular { pat, ty, .. } => {
						if let Type::Path(ty) = &**ty {
							if !type_ends_with(ty, "Option") {
								nargs.0 += 1;
							} else {
								nargs.1 += 1;
							}
						}
						get_ident(&**pat)
					}
					Parameter::This { pat, .. } => {
						if let Pat::Ident(ident) = &**pat {
							this = Some(ident.ident.clone());
							if ident.ident != "self" {
								get_ident(&**pat)
							} else {
								None
							}
						} else {
							None
						}
					}
					Parameter::Context(pat) | Parameter::Arguments(pat) | Parameter::VarArgs { pat, .. } => get_ident(&**pat),
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
		let krate = quote!(::ion);
		let mut index = None;
		let mut args = self
			.parameters
			.iter()
			.enumerate()
			.map(|(i, parameter)| match parameter {
				Parameter::Regular { pat, ty, .. } | Parameter::VarArgs { pat, ty, .. } => parse2(quote_spanned!(pat.span() => #pat: #ty)).unwrap(),
				Parameter::This { pat, ty, kind } => match kind {
					ThisKind::Ref(lt, mutability) => {
						index = Some(i);
						parse2(quote_spanned!(pat.span() => #pat: &#lt #mutability #ty)).unwrap()
					}
					ThisKind::Box => {
						index = Some(i);
						parse2(quote_spanned!(pat.span() => #pat: Box<#ty>)).unwrap()
					}
					ThisKind::Owned => parse2(quote_spanned!(pat.span() => #pat: #ty)).unwrap(),
				},
				Parameter::Context(pat) => parse2(quote_spanned!(pat.span() => #pat: #krate::Context)).unwrap(),
				Parameter::Arguments(pat) => parse2(quote_spanned!(pat.span() => #pat: #krate::Arguments)).unwrap(),
			})
			.collect::<Vec<_>>();
		if let Some(index) = index {
			let this = args.remove(index);
			args.insert(0, this);
		}
		args
	}
}

enum Index {
	Constant(usize),
	Expr(Box<Expr>),
}

fn unwrap_param(index: Index, pat: &Pat, ty: &Type, handle: &Expr, conversion: &Expr) -> Expr {
	let krate = quote!(::ion);
	let pat = format_pat(pat);
	let ty = format_type(ty);
	let error = match index {
		Index::Constant(index) => {
			let error = format!("Failed to convert argument {} at index {}, to {}", pat, index, ty);
			LitStr::new(&error, pat.span()).to_token_stream()
		}
		Index::Expr(index) => {
			let base_error = format!("Failed to convert argument {} at index {{}}, to {}", pat, ty);
			let base_error = LitStr::new(&base_error, pat.span()).to_token_stream();
			quote!(&::std::format!(#base_error, #index))
		}
	};

	parse_quote!({
		if let ::std::option::Option::Some(value) = #krate::types::values::from_value(cx, #handle.get(), #conversion) {
			::std::result::Result::Ok(value)
		} else {
			::std::result::Result::Err(#krate::Error::new(#error, ::std::option::Option::Some(#krate::ErrorKind::Type)))
		}
	})
}

pub(crate) fn get_ident(pat: &Pat) -> Option<Ident> {
	if let Pat::Ident(ident) = pat {
		Some(ident.ident.clone())
	} else {
		None
	}
}

pub(crate) fn parse_this(pat: Box<Pat>, ty: Box<Type>, is_class: bool, span: Span) -> Result<Parameter> {
	match *ty {
		Type::Path(path) if type_ends_with(&path, "Box") => {
			let ty = extract_type_argument(&path, 0).unwrap();
			Ok(Parameter::This { pat, ty, kind: ThisKind::Box })
		}
		Type::Reference(reference) => {
			let ty = reference.elem;
			Ok(Parameter::This {
				pat,
				ty,
				kind: ThisKind::Ref(reference.lifetime, reference.mutability),
			})
		}
		ty if !is_class => Ok(Parameter::This {
			pat,
			ty: Box::new(ty),
			kind: ThisKind::Owned,
		}),
		_ => Err(Error::new(span, "Invalid type for self")),
	}
}
