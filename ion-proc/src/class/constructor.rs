/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use syn::{ItemFn, Result};

use crate::class::method::{Method, MethodReceiver};
use crate::function::{check_abi, set_signature};
use crate::function::parameters::Parameters;
use crate::function::wrapper::impl_wrapper_fn;

pub(crate) fn impl_constructor(mut constructor: ItemFn, ident: &Ident) -> Result<(Method, Parameters)> {
	let krate = quote!(::ion);
	let (wrapper, inner, parameters) = impl_wrapper_fn(constructor.clone(), Some(ident), false, true)?;

	check_abi(&mut constructor)?;
	set_signature(&mut constructor)?;
	constructor.attrs.clear();
	constructor.attrs.push(parse_quote!(#[allow(non_snake_case)]));

	let error_handler = error_handler();

	let body = parse_quote!({
		let args = #krate::Arguments::new(argc, vp);
		#wrapper
		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| wrapper(cx, &args)));
		#error_handler
	});
	constructor.block = body;

	let method = Method {
		receiver: MethodReceiver::Static,
		method: constructor,
		inner: Some(inner),
		nargs: parameters.nargs.0,
		aliases: vec![],
	};
	Ok((method, parameters))
}

pub(crate) fn error_handler() -> TokenStream {
	let krate = quote!(::ion);
	quote!({
		use ::std::prelude::v1::*;

		match result {
			Ok(Ok(_)) => {
				true
			},
			Ok(Err(error)) => {
				use #krate::error::ThrowException;
				error.throw(cx);
				false
			}
			Err(unwind_error) => {
				use #krate::error::ThrowException;
				if let Some(unwind) = unwind_error.downcast_ref::<String>() {
					#krate::Error::new(unwind, None).throw(cx);
				} else if let Some(unwind) = unwind_error.downcast_ref::<&str>() {
					#krate::Error::new(*unwind, None).throw(cx);
				} else {
					#krate::Error::new("Unknown Panic Occurred", None).throw(cx);
					::std::mem::forget(unwind_error);
				}
				false
			}
		}
	})
}
