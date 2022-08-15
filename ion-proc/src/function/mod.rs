/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{Abi, Error, FnArg, ItemFn, Result};
use syn::punctuated::Punctuated;

use crate::function::inner::{DefaultInnerBody, impl_inner_fn};

pub(crate) mod inner;
pub(crate) mod parameters;

pub(crate) fn impl_js_fn(mut function: ItemFn) -> Result<ItemFn> {
	let krate = quote!(::ion);
	let (inner, _, _) = impl_inner_fn::<DefaultInnerBody>(function.clone(), false)?;

	function.attrs = Vec::new();
	check_abi(&mut function)?;
	set_signature(&mut function)?;
	function.attrs.push(parse_quote!(#[allow(non_snake_case)]));

	let error_handler = error_handler();

	let body = parse_quote!({
		let args = #krate::Arguments::new(argc, vp);

		#inner

		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| native_fn(cx, &args)));

		#error_handler
	});
	function.block = Box::new(body);

	Ok(function)
}

pub(crate) fn check_abi(function: &mut ItemFn) -> Result<()> {
	match &function.sig.abi {
		Some(Abi { name: None, .. }) => {}
		None => function.sig.abi = Some(parse_quote!(extern "C")),
		Some(Abi { name: Some(abi), .. }) if abi.value() == "C" => {}
		Some(Abi { name: Some(non_c_abi), .. }) => return Err(Error::new_spanned(non_c_abi, "Expected C ABI")),
	}
	Ok(())
}

pub(crate) fn set_signature(function: &mut ItemFn) -> Result<()> {
	let krate = quote!(::ion);
	function.sig.asyncness = None;
	function.sig.unsafety = Some(parse_quote!(unsafe));
	let params: [FnArg; 3] = [
		parse_quote!(cx: #krate::Context),
		parse_quote!(argc: ::core::primitive::u32),
		parse_quote!(vp: *mut ::mozjs::jsval::JSVal),
	];
	function.sig.inputs = Punctuated::<_, _>::from_iter(params);
	function.sig.output = parse_quote!(-> ::core::primitive::bool);
	Ok(())
}

pub(crate) fn error_handler() -> TokenStream {
	let krate = quote!(::ion);
	quote!({
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
					#krate::Error::new(unwind).throw(cx);
				} else if let Some(unwind) = unwind_error.downcast_ref::<&str>() {
					#krate::Error::new(*unwind).throw(cx);
				} else {
					#krate::Error::new("Unknown Panic Occurred").throw(cx);
					::std::mem::forget(unwind_error);
				}
				false
			}
		}
	})
}
