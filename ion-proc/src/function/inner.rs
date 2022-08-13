/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{Block, FnArg, ItemFn, LitStr, Pat, Stmt, Type};
use syn::punctuated::Punctuated;

use crate::function::parameters::Parameter;
use crate::utils::type_ends_with;

pub(crate) fn extract_params(function: &ItemFn, class: bool) -> syn::Result<(Vec<Stmt>, usize, Option<Ident>)> {
	let mut index = 0;

	let mut nargs = 0;
	let mut this: Option<Ident> = None;

	let statements: Vec<_> = function
		.sig
		.inputs
		.iter()
		.map(|arg| {
			let param = Parameter::from_arg(arg)?;
			match &param {
				Parameter::Normal(ty, _) => {
					if let Type::Path(ty) = &*ty.ty {
						if !type_ends_with(ty, "Option") {
							nargs += 1;
						}
					}
				}
				Parameter::This(pat) => {
					if let Pat::Ident(ident) = &*pat.pat {
						this = Some(ident.ident.clone());
					}
				}
				_ => {}
			}

			if !class {
				Ok(param.into_statement(&mut index))
			} else {
				Ok(param.into_class_statement(&mut index))
			}
		})
		.collect::<syn::Result<_>>()?;

	Ok((statements, nargs, this))
}

pub(crate) fn impl_inner_fn<I: InnerBody>(mut function: ItemFn, class: bool) -> syn::Result<(ItemFn, usize, Option<Ident>)> {
	let krate = quote!(::ion);

	let is_async = function.sig.asyncness.is_some();
	let (params, nargs, this) = extract_params(&function, class)?;
	let input_body = I::impl_inner(function.clone().block, this.clone(), is_async);

	let ident = function.sig.ident.clone();

	function.sig.asyncness = None;
	function.sig.ident = Ident::new("native_fn", function.sig.ident.span());
	let inner_params: [FnArg; 2] = [parse_quote!(cx: #krate::Context), parse_quote!(args: &#krate::Arguments)];
	function.sig.inputs = Punctuated::from_iter(inner_params);

	if is_async {
		function.sig.output = parse_quote!(-> #krate::Result<#krate::Promise>);
	}

	let error_msg = format!(
		"{}() requires at least {} {}",
		ident,
		nargs,
		if nargs == 1 { "argument" } else { "arguments" }
	);
	let error = LitStr::new(&error_msg, ident.span());

	let args_check = if nargs == 0 {
		TokenStream::new()
	} else {
		quote!(
			if args.len() < #nargs {
				return Err(#krate::Error::new(#error));
			}
		)
	};

	let body = parse_quote!({
		#args_check

		#(#params)*

		#input_body
	});
	function.block = Box::new(body);

	Ok((function, nargs, this))
}

pub trait InnerBody {
	fn impl_inner(body: Box<Block>, this: Option<Ident>, is_async: bool) -> TokenStream;
}

pub(crate) struct DefaultInnerBody;

impl InnerBody for DefaultInnerBody {
	fn impl_inner(body: Box<Block>, _: Option<Ident>, is_async: bool) -> TokenStream {
		if !is_async {
			body.into_token_stream()
		} else {
			let krate = quote!(::ion);
			quote! {
				let future = async #body;

				if let Some(promise) = #krate::Promise::new_with_future(cx, future) {
					Ok(promise)
				} else {
					Err(#krate::Error::new("Failed to create Promise"))
				}
			}
		}
	}
}
