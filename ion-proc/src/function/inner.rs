/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{Block, FnArg, ItemFn, LitStr, ReturnType, Signature, Type};
use syn::punctuated::Punctuated;

use crate::function::parameters::extract_params;
use crate::utils::type_ends_with;

pub(crate) fn impl_inner_fn<I: InnerBody>(mut function: ItemFn, class: bool) -> syn::Result<(ItemFn, usize, Option<Ident>)> {
	let krate = quote!(::ion);

	let (params, nargs, this) = extract_params(&function.sig.inputs, class)?;
	let (body, wrapped) = I::impl_inner(function.block.clone(), &function.sig);

	let ident = function.sig.ident.clone();

	if wrapped {
		let output = match function.sig.output.clone() {
			ReturnType::Default => quote!(()),
			ReturnType::Type(_, ty) => ty.to_token_stream(),
		};
		function.sig.output = parse_quote!(-> #krate::Result<#output>);
	}

	if function.sig.asyncness.is_some() {
		function.sig.output = parse_quote!(-> #krate::Result<#krate::Promise>);
	}

	function.sig.asyncness = None;
	function.sig.ident = Ident::new("native_fn", function.sig.ident.span());
	let inner_params: [FnArg; 2] = [parse_quote!(cx: #krate::Context), parse_quote!(args: &#krate::Arguments)];
	function.sig.inputs = Punctuated::from_iter(inner_params);

	let args_check = argument_checker(ident, nargs);

	let body = parse_quote!({
		#args_check
		#(#params)*
		#body
	});
	function.block = Box::new(body);

	Ok((function, nargs, this))
}

pub(crate) fn argument_checker(ident: Ident, nargs: usize) -> TokenStream {
	if nargs != 0 {
		let krate = quote!(::ion);

		let plural = if nargs == 1 { "" } else { "s" };
		let error_msg = format!("{}() requires at least {} argument{}", ident, nargs, plural);
		let error = LitStr::new(&error_msg, ident.span());
		quote!(
			if args.len() < #nargs {
				return Err(#krate::Error::new(#error));
			}
		)
	} else {
		TokenStream::new()
	}
}

pub trait InnerBody {
	fn impl_inner(body: Box<Block>, signature: &Signature) -> (TokenStream, bool);
}

pub(crate) struct DefaultInnerBody;

impl InnerBody for DefaultInnerBody {
	fn impl_inner(body: Box<Block>, signature: &Signature) -> (TokenStream, bool) {
		if signature.asyncness.is_none() {
			let output = &signature.output;

			let wrapped = match output {
				ReturnType::Default => true,
				ReturnType::Type(_, ty) => {
					if let Type::Path(ty) = &**ty {
						!type_ends_with(ty, "Result")
					} else {
						true
					}
				}
			};

			let body = if wrapped {
				quote!(
					let result = (|| #body)();
					Ok(result)
				)
			} else {
				body.to_token_stream()
			};
			(body, wrapped)
		} else {
			let krate = quote!(::ion);
			let body = quote!(
				let future = async move #body;

				if let Some(promise) = ::runtime::promise::future_to_promise(cx, future) {
					Ok(promise)
				} else {
					Err(#krate::Error::new("Failed to create Promise"))
				}
			);
			(body, false)
		}
	}
}
