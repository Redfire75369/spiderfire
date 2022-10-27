/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use prettyplease::unparse;
use proc_macro2::Ident;
use syn::{GenericArgument, GenericParam, Generics, parse2, Pat, PathArguments, Type, TypeParamBound, TypePath};

pub(crate) fn type_ends_with<I: ?Sized>(ty: &TypePath, ident: &I) -> bool
where
	Ident: PartialEq<I>,
{
	if let Some(last) = ty.path.segments.last() {
		&last.ident == ident
	} else {
		false
	}
}

pub(crate) fn extract_last_argument(ty: &Type) -> Option<Ident> {
	if let Type::Path(path) = ty {
		path.path.segments.last().map(|segment| segment.ident.clone())
	} else {
		None
	}
}

pub(crate) fn extract_type_argument(ty: &TypePath, index: usize) -> Option<Box<Type>> {
	if !ty.path.segments.is_empty() && ty.path.segments.len() > index {
		let last = ty.path.segments.last().unwrap();
		if let PathArguments::AngleBracketed(angle_bracketed) = &last.arguments {
			if let Some(GenericArgument::Type(ty)) = angle_bracketed.args.iter().nth(index) {
				return Some(Box::new(ty.clone()));
			}
		}
	}
	None
}

pub(crate) fn add_trait_bounds(generics: &mut Generics, bound: &TypeParamBound) {
	for param in &mut generics.params {
		if let GenericParam::Type(type_param) = param {
			type_param.bounds.push(bound.clone());
		}
	}
}

pub(crate) fn format_type(ty: &Type) -> String {
	let ty = unparse(
		&parse2(quote!(
			impl #ty {}
		))
		.unwrap(),
	);
	let mut ty = String::from(ty.trim());
	ty.drain((ty.len() - 2)..(ty.len()));
	ty.drain(0..4);
	String::from(ty.trim())
}

pub(crate) fn format_pat(pat: &Pat) -> Option<String> {
	let ident = match pat {
		Pat::Ident(ident) => ident.ident.clone(),
		_ => return None,
	};
	let pat = unparse(
		&parse2(quote!(
			const #ident: () = ();
		))
		.unwrap(),
	);
	let mut pat = String::from(pat.trim());
	pat.drain((pat.len() - 10)..(pat.len()));
	pat.drain(0..5);
	Some(String::from(pat.trim()))
}
