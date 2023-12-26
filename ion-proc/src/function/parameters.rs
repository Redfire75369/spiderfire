/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
	Error, Expr, FnArg, GenericArgument, Ident, parse2, Pat, PathArguments, PatIdent, PatType, Receiver, Result, Stmt,
	Type,
};
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit_mut::visit_type_mut;

use crate::attribute::function::ParameterAttribute;
use crate::attribute::ParseAttribute;
use crate::utils::{format_pat, pat_is_ident, path_ends_with};
use crate::visitors::{LifetimeRemover, SelfRenamer};

pub(crate) enum Parameter {
	Regular {
		pat_ty: PatType,
		convert: Box<Expr>,
		strict: bool,
		option: Option<Box<Type>>,
	},
	VarArgs {
		pat_ty: PatType,
		convert: Box<Expr>,
		strict: bool,
	},
	Context(PatType),
	Arguments(PatType),
}

#[derive(Clone)]
pub(crate) enum ThisKind {
	Ref { ty: Box<Type> },
	Object,
	Owned,
}

pub(crate) struct ThisParameter {
	pat_ty: PatType,
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
				let mut pat_ty = pat_ty.clone();
				if let Type::Reference(reference) = &*pat_ty.ty {
					if let Type::Path(path) = &*reference.elem {
						if path_ends_with(&path.path, "Context") {
							return Ok(Parameter::Context(pat_ty));
						} else if path_ends_with(&path.path, "Arguments") {
							return Ok(Parameter::Arguments(pat_ty));
						}
					}
				}

				let mut option = None;

				let attribute = ParameterAttribute::from_attributes_mut("ion", &mut pat_ty.attrs)?;
				let ParameterAttribute { varargs, convert, strict, .. } = attribute;
				let convert = convert.unwrap_or_else(|| parse_quote!(()));

				if let Type::Path(ty) = &*pat_ty.ty {
					if path_ends_with(&ty.path, "Option") {
						let option_segment = ty.path.segments.last().unwrap();
						if let PathArguments::AngleBracketed(inner) = &option_segment.arguments {
							if let GenericArgument::Type(inner) = inner.args.last().unwrap() {
								option = Some(Box::new(inner.clone()));
							}
						}
					}
				}

				if varargs {
					Ok(Parameter::VarArgs { pat_ty, convert, strict })
				} else {
					Ok(Parameter::Regular { pat_ty, convert, strict, option })
				}
			}
			FnArg::Receiver(_) => unreachable!(),
		}
	}

	pub(crate) fn pat_type(&self) -> &PatType {
		use Parameter as P;
		let (P::Regular { pat_ty, .. } | P::VarArgs { pat_ty, .. } | P::Context(pat_ty) | P::Arguments(pat_ty)) = self;
		pat_ty
	}

	pub(crate) fn get_type_without_lifetimes(&self) -> Box<Type> {
		let mut ty = self.pat_type().ty.clone();
		visit_type_mut(&mut LifetimeRemover, &mut ty);
		ty
	}

	pub(crate) fn to_statement(&self, ion: &TokenStream) -> Result<Stmt> {
		use Parameter as P;
		let ty = self.get_type_without_lifetimes();
		let mut pat_ty = self.pat_type().clone();
		pat_ty.attrs.clear();
		match self {
			P::Regular { convert, strict, option, .. } => {
				pat_ty.ty = ty;
				regular_param_statement(ion, &pat_ty, option.as_deref(), convert, *strict)
			}
			P::VarArgs { convert, strict, .. } => {
				let mut pat_ty = pat_ty.clone();
				pat_ty.ty = ty;
				varargs_param_statement(&pat_ty, convert, *strict)
			}
			P::Context(..) => parse2(quote!(let #pat_ty = __cx;)),
			P::Arguments(..) => parse2(quote!(let #pat_ty = __args;)),
		}
	}
}

impl ThisParameter {
	pub(crate) fn from_arg(arg: &FnArg, class_ty: Option<&Type>) -> Result<Option<ThisParameter>> {
		match arg {
			FnArg::Typed(pat_ty) => {
				let span = pat_ty.span();
				let mut pat_ty = pat_ty.clone();
				match class_ty {
					Some(class_ty) if pat_is_ident(&pat_ty.pat, "self") => {
						visit_type_mut(&mut SelfRenamer { ty: class_ty }, &mut pat_ty.ty);
						parse_this(pat_ty, true, span).map(Some)
					}
					_ => {
						let attribute = ParameterAttribute::from_attributes_mut("ion", &mut pat_ty.attrs)?;
						if attribute.this {
							return parse_this(pat_ty, class_ty.is_some(), span).map(Some);
						}
						Ok(None)
					}
				}
			}
			FnArg::Receiver(recv) => {
				if class_ty.is_none() {
					return Err(Error::new(arg.span(), "Can only have self on Class Methods"));
				}

				let class_ty = class_ty.unwrap();
				if recv.colon_token.is_some() {
					return Err(Error::new(arg.span(), "Invalid type for self"));
				}

				let Receiver {
					attrs, reference, mutability, self_token, ..
				} = recv;
				let lifetime = &reference.as_ref().unwrap().1;

				let pat_ty = PatType {
					attrs: attrs.clone(),
					pat: Box::new(Pat::Ident(PatIdent {
						attrs: Vec::new(),
						by_ref: None,
						mutability: None,
						ident: Ident::parse_any.parse2(self_token.to_token_stream()).unwrap(),
						subpat: None,
					})),
					colon_token: <Token![:]>::default(),
					ty: parse2(quote_spanned!(recv.span() => &#lifetime #mutability #class_ty)).unwrap(),
				};
				parse_this(pat_ty, true, recv.span()).map(Some)
			}
		}
	}

	pub(crate) fn to_statement(&self, ion: &TokenStream, is_class: bool) -> Result<Stmt> {
		let ThisParameter { pat_ty, kind } = self;
		let mut pat_ty = pat_ty.clone();
		pat_ty.attrs.clear();

		if is_class && pat_is_ident(&pat_ty.pat, "self") {
			pat_ty.pat = parse_quote!(self_);
		}

		match kind {
			ThisKind::Ref { ty } => {
				let mut ty = ty.clone();
				visit_type_mut(&mut LifetimeRemover, &mut ty);

				if is_class {
					parse2(quote!(
						let #pat_ty = <#ty as #ion::ClassDefinition>::get_mut_private(__this);
					))
				} else {
					parse2(quote!(
						let #pat_ty = <#ty as #ion::conversions::FromValue>::from_value(__cx, __accessor.this(), true, ())?;
					))
				}
			}
			ThisKind::Object => parse2(quote!(let #pat_ty = __this;)),
			ThisKind::Owned => Err(Error::new(pat_ty.span(), "This cannot be owned")),
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
						if let Pat::Ident(ident) = &*this_param.pat_ty.pat {
							if let Some(this) = &this {
								return Some(Err(Error::new(
									this.1.span(),
									"Unable to have multiple this/self parameters",
								)));
							}
							let ident = ident.ident.clone();
							this = Some((this_param, ident, i));
						}
						return None;
					}
					Err(e) => return Some(Err(e)),
					_ => (),
				}

				let param = Parameter::from_arg(arg);
				match &param {
					Ok(Parameter::Regular { pat_ty, option, .. }) => {
						if option.is_none() {
							nargs.0 += 1;
						} else {
							nargs.1 += 1;
						}
						if let Some(ident) = get_ident(&pat_ty.pat) {
							idents.push(ident);
						}
					}
					Ok(Parameter::VarArgs { pat_ty, .. }) => {
						if let Some(ident) = get_ident(&pat_ty.pat) {
							idents.push(ident);
						}
					}
					Ok(Parameter::Context(pat_ty)) => {
						if let Some(ident) = get_ident(&pat_ty.pat) {
							idents.push(ident);
						}
					}
					Ok(Parameter::Arguments(pat_ty)) => {
						if let Some(ident) = get_ident(&pat_ty.pat) {
							idents.push(ident);
						}
					}
					_ => {}
				}
				Some(param)
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
				.map(|parameter| FnArg::Typed(parameter.pat_type().clone()))
				.collect::<Vec<_>>(),
		);

		if let Some((ThisParameter { pat_ty, .. }, _, index)) = &self.this {
			args.insert(*index, FnArg::Typed(pat_ty.clone()));
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

fn regular_param_statement(
	ion: &TokenStream, pat_ty: &PatType, option: Option<&Type>, conversion: &Expr, strict: bool,
) -> Result<Stmt> {
	let not_found_error = if let Some(pat) = format_pat(&pat_ty.pat) {
		format!("Argument {} at index {{}} was not found.", pat)
	} else {
		String::from("Argument at index {{}} was not found.")
	};

	let if_none: Expr = if option.is_some() {
		parse2(quote!(::std::option::Option::None)).unwrap()
	} else {
		parse2(quote!(
			return ::std::result::Result::Err(#ion::Error::new(#not_found_error, #ion::ErrorKind::Type).into())
		))
		.unwrap()
	};

	parse2(quote!(
		let #pat_ty = match unsafe { __accessor.arg(#strict, #conversion) } {
			::std::option::Option::Some(value) => value?,
			::std::option::Option::None => #if_none
		};
	))
}

fn varargs_param_statement(pat_ty: &PatType, conversion: &Expr, strict: bool) -> Result<Stmt> {
	parse2(quote!(let #pat_ty = unsafe { __accessor.args(#strict, #conversion)? };))
}

pub(crate) fn get_ident(pat: &Pat) -> Option<Ident> {
	if let Pat::Ident(ident) = pat {
		Some(ident.ident.clone())
	} else {
		None
	}
}

pub(crate) fn parse_this(pat_ty: PatType, is_class: bool, span: Span) -> Result<ThisParameter> {
	match &*pat_ty.ty {
		Type::Reference(reference) => {
			let elem = reference.clone().elem;
			match &*elem {
				Type::Path(ty) if path_ends_with(&ty.path, "Object") => {
					Ok(ThisParameter { pat_ty, kind: ThisKind::Object })
				}
				_ => Ok(ThisParameter { pat_ty, kind: ThisKind::Ref { ty: elem } }),
			}
		}
		_ if !is_class => Ok(ThisParameter { pat_ty, kind: ThisKind::Owned }),
		_ => Err(Error::new(span, "Invalid type for self")),
	}
}
