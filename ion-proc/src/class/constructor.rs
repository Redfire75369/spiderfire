/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{ItemFn, Result, Type};

use crate::class::method::{Method, MethodReceiver};
use crate::function::{check_abi, set_signature};
use crate::function::wrapper::impl_wrapper_fn;

pub(super) fn impl_constructor(ion: &TokenStream, mut constructor: ItemFn, ty: &Type) -> Result<Method> {
	let (wrapper, parameters) = impl_wrapper_fn(ion, constructor.clone(), Some(ty), true)?;

	check_abi(&mut constructor)?;
	set_signature(&mut constructor)?;
	constructor.attrs.clear();
	constructor.attrs.push(parse_quote!(#[allow(non_snake_case)]));

	let body = parse_quote!({
		let cx = &#ion::Context::new_unchecked(cx);
		let args = &mut #ion::Arguments::new(cx, argc, vp);
		let mut this = #ion::Object::from(
			cx.root(
				::mozjs::jsapi::JS_NewObjectForConstructor(cx.as_ptr(), &<#ty as #ion::ClassDefinition>::class().base, args.call_args())
			)
		);

		#wrapper

		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
			if !args.is_constructing() {
				return ::std::result::Result::Err(#ion::Error::new("Constructor must be called with \"new\".", ::std::option::Option::None).into());
			}

			wrapper(cx, args, &mut this)
		}));

		#ion::function::__handle_native_constructor_result(cx, result, &this, &mut args.rval())
	});
	constructor.block = body;
	constructor.sig.ident = format_ident!("__ion_bindings_constructor", span = constructor.sig.ident.span());

	let method = Method {
		receiver: MethodReceiver::Static,
		method: constructor,
		nargs: parameters.nargs,
		names: vec![],
	};
	Ok(method)
}
