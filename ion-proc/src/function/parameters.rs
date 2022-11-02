/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Span};
use syn::{Error, Expr, FnArg, GenericArgument, Lifetime, parse2, Pat, PathArguments, PatType, Result, Stmt, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit_mut::visit_type_mut;

use crate::function::attribute::ParameterAttribute;
use crate::utils::{extract_type_argument, format_pat, type_ends_with};
use crate::visitors::{LifetimeRemover, SelfRenamer};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ThisKind {
	Ref(Option<Lifetime>, Option<Token![mut]>),
	Box,
	Owned,
}

#[derive(Debug)]
pub(crate) enum Parameter {
	Regular {
		pat: Box<Pat>,
		ty: Box<Type>,
		conversion: Box<Expr>,
		strict: bool,
		option: Option<Box<Type>>,
	},
	VarArgs {
		pat: Box<Pat>,
		ty: Box<Type>,
		conversion: Box<Expr>,
		strict: bool,
	},
	This {
		pat: Box<Pat>,
		ty: Box<Type>,
		kind: ThisKind,
	},
	Context(Box<Pat>, Box<Type>),
	Arguments(Box<Pat>, Box<Type>),
}

pub(crate) struct Parameters {
	pub(crate) parameters: Vec<Parameter>,
	pub(crate) idents: Vec<Ident>,
	pub(crate) nargs: (usize, usize),
	pub(crate) this: Option<Ident>,
}

impl Parameter {
	pub(crate) fn from_arg(arg: &FnArg, class_ty: Option<&Type>) -> Result<Parameter> {
		match arg {
			FnArg::Typed(pat_ty) => {
				let span = pat_ty.span();
				let PatType { pat, ty, .. } = pat_ty.clone();

				if let Type::Reference(reference) = &*ty {
					if let Type::Path(path) = &*reference.elem {
						if type_ends_with(path, "Context") {
							return Ok(Parameter::Context(pat, ty));
						} else if type_ends_with(path, "Arguments") {
							return Ok(Parameter::Arguments(pat, ty));
						}
					}
				}

				match class_ty {
					Some(class_ty) if pat == parse_quote!(self) => {
						let class_ty = Box::new(class_ty.clone());
						let mut self_renamer = SelfRenamer { ty: class_ty };
						let mut ty = ty;
						visit_type_mut(&mut self_renamer, &mut ty);
						parse_this(pat, ty, true, span)
					}
					_ => {
						let mut option = None;
						let mut vararg = false;

						let mut conversion = None;
						let mut strict = false;

						for attr in &pat_ty.attrs {
							if attr.path.is_ident("ion") {
								let args: Punctuated<ParameterAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

								use ParameterAttribute as PA;
								for arg in args {
									match arg {
										PA::This(_) => return parse_this(pat, ty, class_ty.is_some(), span),
										PA::VarArgs(_) => {
											vararg = true;
										}
										PA::Convert { conversion: conversion_expr, .. } => {
											conversion = Some(conversion_expr);
										}
										PA::Strict(_) => {
											strict = true;
										}
									}
								}
							}
						}

						let conversion = conversion.unwrap_or_else(|| parse_quote!(()));

						if let Type::Path(path) = &*ty {
							if type_ends_with(path, "Option") {
								let option_segment = path.path.segments.last().unwrap();
								if let PathArguments::AngleBracketed(inner) = &option_segment.arguments {
									if let GenericArgument::Type(inner) = inner.args.last().unwrap() {
										option = Some(Box::new(inner.clone()));
									}
								}
							}
						}

						if vararg {
							Ok(Parameter::VarArgs { pat, ty, conversion, strict })
						} else {
							Ok(Parameter::Regular { pat, ty, conversion, strict, option })
						}
					}
				}
			}
			FnArg::Receiver(recv) => {
				if class_ty.is_none() {
					return Err(Error::new(arg.span(), "Can only have self on Class Methods"));
				}
				if recv.reference.is_none() {
					return Err(Error::new(arg.span(), "Invalid type for self"));
				}
				let lifetime = recv.reference.as_ref().and_then(|(_, l)| l.as_ref());
				let mutability = recv.mutability;
				let this = <Token![self]>::default();
				let this = parse2(quote!(#this)).unwrap();
				let ty = class_ty.unwrap();

				let ty = parse2(quote!(&#lifetime #mutability #ty)).unwrap();
				parse_this(this, ty, true, recv.span())
			}
		}
	}

	pub(crate) fn get_type_without_lifetimes(&self) -> Type {
		use Parameter as P;
		let (P::Regular { ty, .. } | P::VarArgs { ty, .. } | P::This { ty, .. } | P::Context(_, ty) | P::Arguments(_, ty)) = self;
		let mut ty = *ty.clone();
		let mut lifetime_remover = LifetimeRemover;
		visit_type_mut(&mut lifetime_remover, &mut ty);
		ty
	}

	pub(crate) fn to_statement(&self, index: &mut usize) -> Result<Stmt> {
		let krate = quote!(::ion);
		use Parameter as P;
		let ty = self.get_type_without_lifetimes();
		match self {
			P::Regular { pat, conversion, strict, option, .. } => {
				let value = parse_quote!(args.value(#index));
				*index += 1;
				regular_param_statement(*index - 1, pat, &ty, option.as_deref(), conversion, *strict, &value)
			}
			P::VarArgs { pat, conversion, strict, .. } => varargs_param_statement(*index, pat, &ty, conversion, *strict),
			P::This { pat, .. } => parse2(quote!(
				let #pat: #ty = <#ty as #krate::conversions::FromValue>::from_value(cx, args.this(), true, ())?;
			)),
			P::Context(pat, _) => parse2(quote!(let #pat: #ty = cx;)),
			P::Arguments(pat, _) => parse2(quote!(let #pat: #ty = args;)),
		}
	}

	pub(crate) fn to_class_statement(&self, index: &mut usize) -> Result<Stmt> {
		let krate = quote!(::ion);
		match self {
			Parameter::This { pat, ty, kind } => {
				let pat = if **pat == parse_quote!(self) { parse_quote!(self_) } else { pat.clone() };
				let this = quote!(#krate::Object::from(#krate::Local::from_marked(&args.this().handle().get().to_object())));
				match kind {
					ThisKind::Ref(lt, mutability) => Ok(parse2(quote!(
						let #pat: &#lt #mutability #ty = <#ty as #krate::ClassInitialiser>::get_private(cx, &#this, ::std::option::Option::Some(args))?;
					))?),
					ThisKind::Box => Ok(parse2(quote!(
						let #pat: ::std::boxed::Box<#ty> = <#ty as #krate::ClassInitialiser>::take_private(cx, &#this, ::std::option::Option::Some(args))?;
					))?),
					ThisKind::Owned => Err(Error::new(pat.span(), "Self cannot be owned on Class Methods")),
				}
			}
			param => param.to_statement(index),
		}
	}
}

impl Parameters {
	pub(crate) fn parse(parameters: &Punctuated<FnArg, Token![,]>, ty: Option<&Type>) -> Result<Parameters> {
		let mut nargs = (0, 0);
		let mut this: Option<Ident> = None;
		let mut idents = Vec::new();

		let parameters: Vec<_> = parameters
			.iter()
			.map(|arg| {
				let param = Parameter::from_arg(arg, ty)?;
				let ident = match &param {
					Parameter::Regular { pat, option, .. } => {
						if option.is_none() {
							nargs.0 += 1;
						} else {
							nargs.1 += 1;
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
					Parameter::Context(pat, _) | Parameter::Arguments(pat, _) | Parameter::VarArgs { pat, .. } => get_ident(&**pat),
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
					parameter.to_statement(&mut index)
				} else {
					parameter.to_class_statement(&mut index)
				}
			})
			.collect::<Result<_>>()
	}

	pub(crate) fn to_args(&self) -> Vec<FnArg> {
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
				Parameter::Context(pat, ty) | Parameter::Arguments(pat, ty) => parse2(quote_spanned!(pat.span() => #pat: #ty)).unwrap(),
			})
			.collect::<Vec<_>>();
		if let Some(index) = index {
			let this = args.remove(index);
			args.insert(0, this);
		}
		args
	}
}

fn regular_param_statement(index: usize, pat: &Pat, ty: &Type, option: Option<&Type>, conversion: &Expr, strict: bool, value: &Expr) -> Result<Stmt> {
	let krate = quote!(::ion);

	let pat_str = format_pat(pat);
	let not_found_error = if let Some(pat) = pat_str {
		format!("Argument {} at index {} was not found.", pat, index)
	} else {
		format!("Argument at index {} was not found.", index)
	};
	let if_none: Expr = if option.is_some() {
		parse2(quote!(::std::option::Option::None)).unwrap()
	} else {
		parse2(quote!(return Err(#krate::Error::new(#not_found_error, #krate::ErrorKind::Type).into()))).unwrap()
	};

	parse2(quote!(
		let #pat: #ty = match #value {
			::std::option::Option::Some(value) => <#ty as #krate::conversions::FromValue>::from_value(cx, value, #strict, #conversion)?,
			::std::option::Option::None => #if_none,
		};
	))
}

fn varargs_param_statement(start_index: usize, pat: &Pat, ty: &Type, conversion: &Expr, strict: bool) -> Result<Stmt> {
	let krate = quote!(::ion);

	parse2(quote!(
		let #pat: #ty = args.range(#start_index..=args.len()).into_iter().map(|value| {
			#krate::conversions::FromValue::from_value(cx, value, #strict, #conversion)
		}).collect::<#krate::Result<_>>()?;
	))
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
