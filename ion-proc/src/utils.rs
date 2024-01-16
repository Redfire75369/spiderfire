/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use prettyplease::unparse;
use proc_macro2::Ident;
use syn::{GenericParam, Generics, parse2, Pat, Path, Type, TypeParamBound};

pub(crate) fn path_ends_with<I: ?Sized>(path: &Path, ident: &I) -> bool
where
	Ident: PartialEq<I>,
{
	if let Some(last) = path.segments.last() {
		&last.ident == ident
	} else {
		false
	}
}

pub(crate) fn pat_is_ident<I: ?Sized>(pat: &Pat, ident: &I) -> bool
where
	Ident: PartialEq<I>,
{
	if let Pat::Ident(pat) = pat {
		&pat.ident == ident
	} else {
		false
	}
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

macro_rules! new_token {
    [$i:tt] => {
		<Token![$i]>::default()
    };
}

pub(crate) use new_token;
