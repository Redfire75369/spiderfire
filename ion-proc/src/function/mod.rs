/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{Abi, Block, Error, FnArg, Generics, ItemFn, parse2, Result};
use syn::punctuated::Punctuated;

use crate::attribute::krate::crate_from_attributes;
use crate::function::wrapper::impl_wrapper_fn;
use crate::utils::new_token;

pub(crate) mod inner;
pub(crate) mod parameter;
pub(crate) mod wrapper;

// TODO: Partially Remove Error Handling in Infallible Functions
pub(crate) fn impl_js_fn(mut function: ItemFn) -> Result<ItemFn> {
	let ion = &crate_from_attributes(&mut function.attrs);
	let (wrapper, _) = impl_wrapper_fn(ion, function.clone(), None, false)?;

	check_abi(&mut function)?;
	set_signature(&mut function)?;

	function.attrs.clear();
	function.attrs.push(parse_quote!(#[allow(non_snake_case)]));
	function.block = impl_fn_body(ion, &wrapper)?;

	Ok(function)
}

pub(crate) fn check_abi(function: &mut ItemFn) -> Result<()> {
	match &function.sig.abi {
		None => function.sig.abi = Some(parse_quote!(extern "C")),
		Some(Abi { name: None, .. }) => {}
		Some(Abi { name: Some(abi), .. }) if abi.value() == "C" => {}
		Some(Abi { name: Some(non_c_abi), .. }) => return Err(Error::new_spanned(non_c_abi, "Expected C ABI")),
	}
	Ok(())
}

pub(crate) fn set_signature(function: &mut ItemFn) -> Result<()> {
	function.sig.unsafety = Some(new_token![unsafe]);
	let params: [FnArg; 3] = [
		parse_quote!(cx: *mut ::mozjs::jsapi::JSContext),
		parse_quote!(argc: ::core::primitive::u32),
		parse_quote!(vp: *mut ::mozjs::jsval::JSVal),
	];
	function.sig.generics = Generics::default();
	function.sig.inputs = Punctuated::from_iter(params);
	function.sig.output = parse_quote!(-> ::core::primitive::bool);
	Ok(())
}

pub(crate) fn impl_fn_body(ion: &TokenStream, wrapper: &ItemFn) -> Result<Box<Block>> {
	parse2(quote!({
		let cx = &#ion::Context::new_unchecked(cx);
		let args = &mut #ion::Arguments::new(cx, argc, vp);

		#wrapper
		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| wrapper(cx, args)));
		#ion::function::__handle_native_function_result(cx, result)
	}))
}
