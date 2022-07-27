/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Abi, Error, FnArg, ItemFn};
use syn::punctuated::Punctuated;

use crate::function::inner::impl_inner_fn;

mod inner;
mod parameters;

pub(crate) fn impl_js_fn(function: ItemFn) -> syn::Result<TokenStream> {
	let krate = quote!(::ion);
	let mut outer = function.clone();

	match &function.sig.abi {
		Some(Abi { name: None, .. }) => {}
		None => outer.sig.abi = Some(parse_quote!(extern "C")),
		Some(Abi { name: Some(abi), .. }) if abi.value() == "C" => {}
		Some(Abi { name: Some(non_c_abi), .. }) => return Err(Error::new_spanned(non_c_abi, "Expected C ABI")),
	}

	let inner = impl_inner_fn(&function)?;

	outer.attrs.push(parse_quote!(#[allow(non_snake_case)]));

	outer.sig.asyncness = None;
	outer.sig.unsafety = Some(parse_quote!(unsafe));
	let outer_params: [FnArg; 3] = [
		parse_quote!(cx: #krate::Context),
		parse_quote!(argc: ::core::primitive::u32),
		parse_quote!(vp: *mut ::mozjs::jsval::JSVal),
	];
	outer.sig.inputs = Punctuated::<_, _>::from_iter(outer_params);
	outer.sig.output = parse_quote!(-> ::core::primitive::bool);

	let body = quote!({
		let args = #krate::Arguments::new(argc, vp);

		#inner

		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| native_fn(cx, &args)));

		{
			use ::std::prelude::v1::*;

			match result {
				Ok(Ok(v)) => {
					use ::mozjs::conversions::ToJSValConvertible;
					v.to_jsval(cx, ::mozjs::rust::MutableHandle::from_raw(args.rval()));
					true
				},
				Ok(Err(error)) => {
					error.throw(cx);
					false
				}
				Err(unwind_error) => {
					if let Some(unwind) = unwind_error.downcast_ref::<String>() {
						#krate::Error::Error(unwind.clone()).throw(cx);
					} else if let Some(unwind) = unwind_error.downcast_ref::<&str>() {
						#krate::Error::Error(String::from(*unwind)).throw(cx);
					} else {
						#krate::Error::Error(String::from("Unknown Panic Occurred")).throw(cx);
						::std::mem::forget(unwind_error);
					}
					false
				}
			}
		}
	});
	outer.block = parse_quote!(#body);

	Ok(outer.into_token_stream())
}
