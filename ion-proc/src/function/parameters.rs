/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Span, TokenStream};
use syn::{Error, Expr, FnArg, GenericArgument, Ident, Lifetime, parse2, Pat, PathArguments, PatType, Result, Stmt, Type};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit_mut::visit_type_mut;

use crate::attribute::function::ParameterAttribute;
use crate::utils::{format_pat, path_ends_with};
use crate::visitors::{LifetimeRemover, SelfRenamer};

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
	Context(Box<Pat>, Box<Type>),
	Arguments(Box<Pat>, Box<Type>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ThisKind {
	Ref(Option<Lifetime>, Option<Token![mut]>),
	Object(Option<Lifetime>, Option<Token![mut]>),
	Owned,
}

#[derive(Debug)]
pub(crate) struct ThisParameter {
	pat: Box<Pat>,
	ty: Box<Type>,
	kind: ThisKind,
}

pub(crate) struct Parameters {
	pub(crate) parameters: Vec<Parameter>,
	pub(crate) this: Option<(ThisParameter, Ident, usize)>,
	pub(crate) idents: Vec<Ident>,
	pub(crate) nargs: (usize, usize),
}

impl Parameter {
	pub(crate) fn from_arg(arg: &FnArg) -> Result<Parameter> {
		match arg {
			FnArg::Typed(pat_ty) => {
				let PatType { pat, ty, .. } = pat_ty.clone();

				if let Type::Reference(reference) = &*ty {
					if let Type::Path(path) = &*reference.elem {
						if path_ends_with(&path.path, "Context") {
							return Ok(Parameter::Context(pat, ty));
						} else if path_ends_with(&path.path, "Arguments") {
							return Ok(Parameter::Arguments(pat, ty));
						}
					}
				}

				let mut option = None;
				let mut vararg = false;

				let mut conversion = None;
				let mut strict = false;

				for attr in &pat_ty.attrs {
					if attr.path().is_ident("ion") {
						let args: Punctuated<ParameterAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;

						use ParameterAttribute as PA;
						for arg in args {
							match arg {
								PA::VarArgs(_) => {
									vararg = true;
								}
								PA::Convert(convert) => {
									conversion = Some(convert.conversion);
								}
								PA::Strict(_) => {
									strict = true;
								}
								_ => (),
							}
						}
					}
				}

				let conversion = conversion.unwrap_or_else(|| parse_quote!(()));

				if let Type::Path(ty) = &*ty {
					if path_ends_with(&ty.path, "Option") {
						let option_segment = ty.path.segments.last().unwrap();
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
			FnArg::Receiver(_) => unreachable!(),
		}
	}

	pub(crate) fn get_type_without_lifetimes(&self) -> Type {
		use Parameter as P;
		let (P::Regular { ty, .. } | P::VarArgs { ty, .. } | P::Context(_, ty) | P::Arguments(_, ty)) = self;
		let mut ty = *ty.clone();
		visit_type_mut(&mut LifetimeRemover, &mut ty);
		ty
	}

	pub(crate) fn to_statement(&self, ion: &TokenStream) -> Result<Stmt> {
		use Parameter as P;
		let ty = self.get_type_without_lifetimes();
		match self {
			P::Regular { pat, conversion, strict, option, .. } => regular_param_statement(ion, pat, &ty, option.as_deref(), conversion, *strict),
			P::VarArgs { pat, conversion, strict, .. } => varargs_param_statement(pat, &ty, conversion, *strict),
			P::Context(pat, _) => parse2(quote!(let #pat: #ty = __cx;)),
			P::Arguments(pat, _) => parse2(quote!(let #pat: #ty = __args;)),
		}
	}
}

impl ThisParameter {
	pub(crate) fn from_arg(arg: &FnArg, class_ty: Option<&Type>) -> Result<Option<ThisParameter>> {
		match arg {
			FnArg::Typed(pat_ty) => {
				let span = pat_ty.span();
				let PatType { pat, mut ty, .. } = pat_ty.clone();
				match class_ty {
					Some(class_ty) if pat == parse_quote!(self) => {
						visit_type_mut(&mut SelfRenamer { ty: class_ty }, &mut ty);
						parse_this(pat, ty, true, span).map(Some)
					}
					_ => {
						for attr in &pat_ty.attrs {
							if attr.path().is_ident("ion") {
								let args: Punctuated<ParameterAttribute, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;
								for arg in args {
									if let ParameterAttribute::This(_) = arg {
										return parse_this(pat, ty, class_ty.is_some(), span).map(Some);
									}
								}
							}
						}
						Ok(None)
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
				let parser = Pat::parse_single;
				let this = Box::new(parser.parse2(quote!(#this)).unwrap());
				let ty = class_ty.unwrap();

				let ty = parse2(quote!(&#lifetime #mutability #ty)).unwrap();
				parse_this(this, ty, true, recv.span()).map(Some)
			}
		}
	}

	pub(crate) fn to_statement(&self, ion: &TokenStream, is_class: bool) -> Result<Stmt> {
		let ThisParameter { pat, ty, kind } = self;
		let self_ = parse_quote!(self_);
		let pat = if is_class && **pat == parse_quote!(self) { &self_ } else { pat };
		let mut ty = ty.clone();
		visit_type_mut(&mut LifetimeRemover, &mut ty);

		match kind {
			ThisKind::Ref(lt, mutability) => {
				if is_class {
					parse2(quote!(
						let #pat: &#lt #mutability #ty = <#ty as #ion::ClassDefinition>::get_mut_private(__this);
					))
				} else {
					parse2(quote!(
						let #pat: #ty = <#ty as #ion::conversions::FromValue>::from_value(__cx, __accessor.this(), true, ())?;
					))
				}
			}
			ThisKind::Object(_, mutability) => parse2(quote!(let #pat: &#mutability #ty = __this;)),
			ThisKind::Owned => Err(Error::new(pat.span(), "This cannot be owned")),
		}
	}
}

impl Parameters {
	pub(crate) fn parse(parameters: &Punctuated<FnArg, Token![,]>, ty: Option<&Type>) -> Result<Parameters> {
		let mut nargs = (0, 0);
		let mut this: Option<(ThisParameter, Ident, usize)> = None;
		let mut idents = Vec::new();

		let parameters: Vec<_> = parameters
			.iter()
			.enumerate()
			.filter_map(|(i, arg)| {
				let this_param = ThisParameter::from_arg(arg, ty);
				match this_param {
					Ok(Some(this_param)) => {
						if let Pat::Ident(ident) = (*this_param.pat).clone() {
							if let Some(this) = &this {
								return Some(Err(Error::new(this.1.span(), "Unable to have multiple this/self parameters")));
							}
							this = Some((this_param, ident.ident.clone(), i));
						}
						return None;
					}
					Err(e) => return Some(Err(e)),
					_ => (),
				}
				let param = Parameter::from_arg(arg);
				match param {
					Ok(Parameter::Regular { pat, ty, conversion, strict, option }) => {
						if option.is_none() {
							nargs.0 += 1;
						} else {
							nargs.1 += 1;
						}
						if let Some(ident) = get_ident(&pat) {
							idents.push(ident);
						}
						Some(Ok(Parameter::Regular { pat, ty, conversion, strict, option }))
					}
					Ok(Parameter::VarArgs { pat, ty, conversion, strict }) => {
						if let Some(ident) = get_ident(&pat) {
							idents.push(ident);
						}
						Some(Ok(Parameter::VarArgs { pat, ty, conversion, strict }))
					}
					Ok(Parameter::Context(pat, ty)) => {
						if let Some(ident) = get_ident(&pat) {
							idents.push(ident);
						}
						Some(Ok(Parameter::Context(pat, ty)))
					}
					Ok(Parameter::Arguments(pat, ty)) => {
						if let Some(ident) = get_ident(&pat) {
							idents.push(ident);
						}
						Some(Ok(Parameter::Arguments(pat, ty)))
					}
					Err(e) => Some(Err(e)),
				}
			})
			.collect::<Result<_>>()?;

		Ok(Parameters { parameters, this, idents, nargs })
	}

	pub(crate) fn to_statements(&self, ion: &TokenStream) -> Result<Vec<Stmt>> {
		self.parameters.iter().map(|parameter| parameter.to_statement(ion)).collect()
	}

	pub(crate) fn get_this_ident(&self) -> Option<Ident> {
		self.this.as_ref().map(|x| x.1.clone())
	}

	pub(crate) fn to_this_statement(&self, ion: &TokenStream, is_class: bool) -> Result<Option<Stmt>> {
		match &self.this {
			Some((this, _, _)) => Ok(Some(this.to_statement(ion, is_class)?)),
			None => Ok(None),
		}
	}

	pub(crate) fn to_args(&self) -> Vec<FnArg> {
		let mut args = Vec::with_capacity(self.this.is_some() as usize + self.parameters.len());

		args.extend(
			self.parameters
				.iter()
				.map(|parameter| match parameter {
					Parameter::Regular { pat, ty, .. } | Parameter::VarArgs { pat, ty, .. } => {
						parse2(quote_spanned!(pat.span() => #pat: #ty)).unwrap()
					}
					Parameter::Context(pat, ty) | Parameter::Arguments(pat, ty) => parse2(quote_spanned!(pat.span() => #pat: #ty)).unwrap(),
				})
				.collect::<Vec<_>>(),
		);

		if let Some((ThisParameter { pat, ty, kind }, _, index)) = &self.this {
			args.insert(
				*index,
				match kind {
					ThisKind::Ref(lt, mutability) => parse2(quote_spanned!(pat.span() => #pat: &#lt #mutability #ty)).unwrap(),
					ThisKind::Object(lt, mutability) => parse2(quote_spanned!(pat.span() => #pat: &#lt #mutability #ty)).unwrap(),
					ThisKind::Owned => parse2(quote_spanned!(pat.span() => #pat: #ty)).unwrap(),
				},
			);
		}

		args
	}

	pub(crate) fn to_idents(&self) -> Vec<Ident> {
		let mut idents = self.idents.clone();
		if let Some((_, ident, index)) = &self.this {
			if ident != &Into::<Ident>::into(<Token![self]>::default()) {
				idents.insert(*index, ident.clone());
			}
		}
		idents
	}
}

fn regular_param_statement(ion: &TokenStream, pat: &Pat, ty: &Type, option: Option<&Type>, conversion: &Expr, strict: bool) -> Result<Stmt> {
	let pat_str = format_pat(pat);
	let not_found_error = if let Some(pat) = pat_str {
		format!("Argument {} at index {{}} was not found.", pat)
	} else {
		String::from("Argument at index {{}} was not found.")
	};
	let if_none: Expr = if option.is_some() {
		parse2(quote!(::std::option::Option::None)).unwrap()
	} else {
		parse2(quote!(return Err(#ion::Error::new(#not_found_error, #ion::ErrorKind::Type).into()))).unwrap()
	};

	parse2(quote!(
		let #pat: #ty = match unsafe { __accessor.arg(#strict, #conversion) } {
			::std::option::Option::Some(value) => value?,
			::std::option::Option::None => #if_none
		};
	))
}

fn varargs_param_statement(pat: &Pat, ty: &Type, conversion: &Expr, strict: bool) -> Result<Stmt> {
	parse2(quote!(let #pat: #ty = unsafe { __accessor.args(#strict, #conversion)? };))
}

pub(crate) fn get_ident(pat: &Pat) -> Option<Ident> {
	if let Pat::Ident(ident) = pat {
		Some(ident.ident.clone())
	} else {
		None
	}
}

pub(crate) fn parse_this(pat: Box<Pat>, ty: Box<Type>, is_class: bool, span: Span) -> Result<ThisParameter> {
	match *ty {
		Type::Reference(reference) => {
			let lt = reference.lifetime;
			let mutability = reference.mutability;
			match *reference.elem {
				Type::Path(ty) if path_ends_with(&ty.path, "Object") => Ok(ThisParameter {
					pat,
					ty: Box::new(Type::Path(ty)),
					kind: ThisKind::Object(lt, mutability),
				}),
				ty => Ok(ThisParameter {
					pat,
					ty: Box::new(ty),
					kind: ThisKind::Ref(lt, mutability),
				}),
			}
		}
		ty if !is_class => Ok(ThisParameter {
			pat,
			ty: Box::new(ty),
			kind: ThisKind::Owned,
		}),
		_ => Err(Error::new(span, "Invalid type for self")),
	}
}
