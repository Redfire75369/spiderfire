/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{Block, FnArg, ItemFn, Stmt};
use syn::punctuated::Punctuated;

use crate::function::parameters::Parameter;

fn extract_params(function: &ItemFn) -> syn::Result<(Vec<Stmt>, usize)> {
	let mut nargs = 0;
	let mut index = 0;
	let statements: Vec<_> = function
		.sig
		.inputs
		.iter()
		.map(|arg| {
			let param = Parameter::from_arg(arg)?;
			if param.is_normal() {
				nargs += 1;
			}
			Ok(param.into_statement(&mut index))
		})
		.collect::<syn::Result<_>>()?;

	Ok((statements, nargs))
}

pub(crate) fn impl_inner_fn(function: &ItemFn) -> syn::Result<ItemFn> {
	let krate = quote!(::ion);
	let mut inner = function.clone();

	let is_async = function.sig.asyncness.is_some();
	let (params, nargs) = extract_params(function)?;

	inner.sig.asyncness = None;
	inner.sig.ident = Ident::new("native_fn", function.sig.ident.span());
	let inner_params: [FnArg; 2] = [parse_quote!(cx: #krate::Context), parse_quote!(args: &#krate::Arguments)];
	inner.sig.inputs = Punctuated::from_iter(inner_params);

	if is_async {
		inner.sig.output = parse_quote!(-> #krate::Result<#krate::Promise>);
	}

	let fn_name = function.sig.ident.to_string();
	let input_body = impl_inner_body(is_async, function.clone().block);
	let inner_body = parse_quote!({
		if args.len() < #nargs {
			return Err(#krate::Error::Error(::std::format!("{}() requires at least {} {}", #fn_name,
				#nargs, if #nargs == 1 { "argument" } else { "arguments" })));
		}

		#(#params)*

		#input_body
	});
	inner.block = Box::new(inner_body);

	Ok(inner)
}

fn impl_inner_body(is_async: bool, body: Box<Block>) -> TokenStream {
	if !is_async {
		body.into_token_stream()
	} else {
		impl_async_body(body)
	}
}

fn impl_async_body(body: Box<Block>) -> TokenStream {
	let krate = quote!(::ion);
	quote! {
		let future = async #body;

		if let Some(promise) = #krate::Promise::new_with_future(cx, future) {
			Ok(promise)
		} else {
			Err(#krate::Error::None)
		}
	}
}
