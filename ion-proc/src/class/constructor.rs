/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{ItemFn, Result, Type};

use crate::class::method::{Method, MethodReceiver};
use crate::function::wrapper::impl_wrapper_fn;
use crate::function::{check_abi, set_signature};

pub(super) fn impl_constructor(ion: &TokenStream, mut constructor: ItemFn, ty: &Type) -> Result<Method> {
	let (wrapper, parameters) = impl_wrapper_fn(ion, constructor.clone(), Some(ty), true)?;

	check_abi(&mut constructor)?;
	set_signature(&mut constructor)?;
	constructor.attrs.clear();

	let body = parse_quote!({
		let cx = &#ion::Context::new_unchecked(cx);
		let args = &mut #ion::Arguments::new(cx, argc, vp);
		let mut this = #ion::Object::from(
			cx.root(
				::mozjs::jsapi::JS_NewObjectForConstructor(cx.as_ptr(), &<#ty as #ion::ClassDefinition>::class().base, args.call_args())
			)
		);

		#wrapper

		if !args.is_constructing() {
			let name = unsafe {
				::std::ffi::CStr::from_ptr(<#ty as #ion::ClassDefinition>::class().base.name).to_str().unwrap()
			};
			::mozjs::error::throw_type_error(cx.as_ptr(), &format!("{name} constructor: 'new' is required"));
			return false;
		}

		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
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
