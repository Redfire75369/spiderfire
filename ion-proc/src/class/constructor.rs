/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{ItemFn, Result, Type};

use crate::class::method::{Method, MethodReceiver};
use crate::function::{check_abi, set_signature};
use crate::function::parameters::Parameters;
use crate::function::wrapper::impl_wrapper_fn;

pub(crate) fn impl_constructor(mut constructor: ItemFn, ty: &Type) -> Result<(Method, Parameters)> {
	let krate = quote!(::ion);
	let (wrapper, inner, parameters) = impl_wrapper_fn(constructor.clone(), Some(ty), false)?;

	check_abi(&mut constructor)?;
	set_signature(&mut constructor)?;
	constructor.attrs.clear();
	constructor.attrs.push(parse_quote!(#[allow(non_snake_case)]));

	let error_handler = error_handler();

	let body = parse_quote!({
		let cx = &#krate::Context::new(&mut cx);
		let mut args = #krate::Arguments::new(cx, argc, vp);

		#wrapper
		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
			if !args.is_constructing() {
				return ::std::result::Result::Err(#krate::Error::new("Constructor must be called with \"new\".", ::std::option::Option::None).into());
			}

			wrapper(cx, &mut args)
		}));
		#error_handler
	});
	constructor.block = body;

	let method = Method {
		receiver: MethodReceiver::Static,
		method: constructor,
		inner: Some(inner),
		nargs: parameters.nargs.0,
		names: vec![],
	};
	Ok((method, parameters))
}

pub(crate) fn error_handler() -> TokenStream {
	let krate = quote!(::ion);
	quote!({
		use ::std::prelude::v1::*;

		match result {
			Ok(Ok(_)) => {
				let b = ::std::boxed::Box::new(result);
				let this = #krate::Object::from(cx.root_object(::mozjs::jsapi::JS_NewObjectForConstructor(**cx, &CLASS, &args.call_args())));
				::mozjs::jsapi::SetPrivate(**this, Box::into_raw(b) as *mut ::std::ffi::c_void);
				#krate::conversions::ToValue::to_value(&this, cx, args.rval());
				true
			},
			Ok(Err(error)) => {
				#krate::error::ThrowException::throw(&error, cx);
				false
			}
			Err(unwind_error) => {
				if let Some(unwind) = unwind_error.downcast_ref::<String>() {
					#krate::error::ThrowException::throw(&#krate::Error::new(unwind, ::std::option::Option::None), cx);
				} else if let Some(unwind) = unwind_error.downcast_ref::<&str>() {
					#krate::error::ThrowException::throw(&#krate::Error::new(*unwind, ::std::option::Option::None), cx);
				} else {
					#krate::error::ThrowException::throw(&#krate::Error::new("Unknown Panic Occurred", None), cx);
					::std::mem::forget(unwind_error);
				}
				false
			}
		}
	})
}
