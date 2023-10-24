/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{Error, FnArg, GenericParam, ItemFn, parse2, PathArguments, Result, ReturnType, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::function::inner::impl_inner_fn;
use crate::function::parameters::Parameters;
use crate::utils::path_ends_with;

pub(crate) fn impl_wrapper_fn(
	ion: &TokenStream, mut function: ItemFn, class_ty: Option<&Type>, is_constructor: bool,
) -> Result<(ItemFn, Parameters)> {
	if function.sig.asyncness.is_some() {
		return Err(Error::new(
			function.sig.asyncness.span(),
			"Async functions cannot be used as methods. Use `Promise::block_on_future` or `future_to_promise` instead.",
		));
	}

	let parameters = Parameters::parse(&function.sig.inputs, class_ty)?;
	let idents = parameters.to_idents();
	let statements = parameters.to_statements(ion)?;

	let inner = impl_inner_fn(function.clone(), &parameters, class_ty.is_none());

	let wrapper_generics: [GenericParam; 2] = [parse_quote!('cx), parse_quote!('a)];
	let mut wrapper_args: Vec<FnArg> = vec![
		parse_quote!(__cx: &'cx #ion::Context),
		parse_quote!(__args: &'a mut #ion::Arguments<'_, 'cx>),
	];

	let argument_checker = argument_checker(ion, &function.sig.ident, parameters.nargs.0);

	let mut this_statements = parameters.to_this_statement(ion, class_ty.is_some())?.map(ToTokens::into_token_stream);
	if is_constructor {
		wrapper_args.push(parse_quote!(__this: &mut #ion::Object<'cx>));
	} else {
		this_statements = this_statements.map(|statement| {
			quote!(
				let __this = &mut __accessor.this_mut().to_object(__cx);
				#statement
			)
		});
	}

	let output = match &function.sig.output {
		ReturnType::Default => parse_quote!(()),
		ReturnType::Type(_, ty) => *ty.clone(),
	};
	let mut wrapper_output = output.clone();

	let mut result = quote!(__result.map_err(::std::convert::Into::into));

	if let Type::Path(ty) = &output {
		if !path_ends_with(&ty.path, "ResultExc") {
			if path_ends_with(&ty.path, "Result") {
				if let PathArguments::AngleBracketed(args) = &ty.path.segments.last().unwrap().arguments {
					let arg = args.args.first().unwrap();
					wrapper_output = parse_quote!(#ion::ResultExc<#arg>);
				}
			} else {
				result = quote!(#ion::ResultExc::<#ty>::Ok(__result));
				wrapper_output = parse_quote!(#ion::ResultExc::<#ty>);
			}
		}
	} else {
		result = quote!(#ion::ResultExc::<#output>::Ok(__result));
		wrapper_output = parse_quote!(#ion::ResultExc::<#output>);
	}

	let wrapper_inner = class_ty.is_none().then_some(&inner);

	let ident = &function.sig.ident;
	let call = if let Some(class) = class_ty {
		quote!(#class::#ident)
	} else {
		quote!(inner)
	};

	let inner_call = if parameters.get_this_ident() == Some(<Token![self]>::default().into()) {
		quote!(#call(self_, #(#idents),*))
	} else {
		quote!(#call(#(#idents),*))
	};

	let body = parse2(quote_spanned!(function.span() => {
		#argument_checker

		let mut __accessor = __args.access();
		#this_statements
		#(#statements)*

		#wrapper_inner

		#[allow(clippy::let_unit_value)]
		let __result: #output = #inner_call;
		#result
	}))?;

	function.sig.ident = format_ident!("wrapper", span = function.sig.ident.span());
	function.sig.inputs = Punctuated::from_iter(wrapper_args);
	function.sig.generics.params = Punctuated::from_iter(wrapper_generics);
	function.sig.output = parse_quote!(-> #wrapper_output);
	function.sig.unsafety = Some(<Token![unsafe]>::default());

	function.attrs.clear();
	function.block = body;

	Ok((function, parameters))
}

pub(crate) fn argument_checker(ion: &TokenStream, ident: &Ident, nargs: usize) -> TokenStream {
	if nargs != 0 {
		let plural = if nargs == 1 { "" } else { "s" };
		let error = format!("{}() requires at least {} argument{}", ident, nargs, plural);
		quote!(
			if __args.len() < #nargs {
				return ::std::result::Result::Err(#ion::Error::new(#error, ::std::option::Option::None).into());
			}
		)
	} else {
		TokenStream::new()
	}
}
