/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{Abi, Error, FnArg, Generics, ItemFn, Result};
use syn::punctuated::Punctuated;

use crate::function::wrapper::impl_wrapper_fn;

pub(crate) mod inner;
pub(crate) mod parameters;
pub(crate) mod wrapper;

// TODO: Partially Remove Error Handling in Infallible Functions
pub(crate) fn impl_js_fn(mut function: ItemFn) -> Result<ItemFn> {
	let krate = quote!(::ion);
	let (wrapper, _, _) = impl_wrapper_fn(function.clone(), None, true, false)?;

	check_abi(&mut function)?;
	set_signature(&mut function)?;
	function.attrs.clear();
	function.attrs.push(parse_quote!(#[allow(non_snake_case)]));

	let error_handler = error_handler();

	let body = parse_quote!({
		let cx = &#krate::Context::new(&mut cx);
		let mut args = #krate::Arguments::new(cx, argc, vp);

		#wrapper
		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| wrapper(cx, &mut args)));
		#error_handler
	});
	function.block = Box::new(body);

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
	function.sig.asyncness = None;
	function.sig.unsafety = Some(<Token![unsafe]>::default());
	let params: [FnArg; 3] = [
		parse_quote!(mut cx: *mut ::mozjs::jsapi::JSContext),
		parse_quote!(argc: ::core::primitive::u32),
		parse_quote!(vp: *mut ::mozjs::jsval::JSVal),
	];
	function.sig.generics = Generics::default();
	function.sig.inputs = Punctuated::<_, _>::from_iter(params);
	function.sig.output = parse_quote!(-> ::core::primitive::bool);
	Ok(())
}

pub(crate) fn error_handler() -> TokenStream {
	let krate = quote!(::ion);
	quote!(
		match result {
			::std::result::Result::Ok(::std::result::Result::Ok(val)) => {
				#krate::conversions::ToValue::to_value(&val, cx, &mut args.rval());
				true
			},
			::std::result::Result::Ok(::std::result::Result::Err(error)) => {
				#krate::exception::ThrowException::throw(&error, cx);
				false
			}
			::std::result::Result::Err(unwind_error) => {
				if let ::std::option::Option::Some(unwind) = unwind_error.downcast_ref::<String>() {
					#krate::exception::ThrowException::throw(&#krate::Error::new(unwind, ::std::option::Option::None), cx);
				} else if let ::std::option::Option::Some(unwind) = unwind_error.downcast_ref::<&str>() {
					#krate::exception::ThrowException::throw(&#krate::Error::new(*unwind, ::std::option::Option::None), cx);
				} else {
					#krate::exception::ThrowException::throw(&#krate::Error::new("Unknown Panic Occurred", ::std::option::Option::None), cx);
					::std::mem::forget(unwind_error);
				}
				false
			}
		}
	)
}
