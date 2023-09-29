/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{GenericArgument, ItemFn, PathArguments, Result, ReturnType, Type};

use crate::attribute::krate::Crates;
use crate::class::method::{Method, MethodReceiver};
use crate::function::{check_abi, set_signature};
use crate::function::parameters::Parameters;
use crate::function::wrapper::impl_wrapper_fn;
use crate::utils::type_ends_with;

pub(crate) fn impl_constructor(crates: &Crates, mut constructor: ItemFn, ty: &Type) -> Result<(Method, Parameters)> {
	let (wrapper, inner, parameters) = impl_wrapper_fn(crates, constructor.clone(), Some(ty), false, true)?;

	let ion = &crates.ion;

	check_abi(&mut constructor)?;
	set_signature(&mut constructor)?;
	constructor.attrs.clear();
	constructor.attrs.push(parse_quote!(#[allow(non_snake_case)]));

	let empty = Box::new(parse_quote!(()));
	let return_type = match &wrapper.sig.output {
		ReturnType::Default => &empty,
		ReturnType::Type(_, ty) => ty,
	};
	let error_handler = error_handler(ion, ty, return_type);

	let body = parse_quote!({
		let cx = &#ion::Context::new_unchecked(cx);
		let args = &mut #ion::Arguments::new(cx, argc, vp);
		let mut this = #ion::Object::from(cx.root_object(::mozjs::jsapi::JS_NewObjectForConstructor(cx.as_ptr(), &CLASS, &args.call_args())));

		#wrapper
		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
			if !args.is_constructing() {
				return ::std::result::Result::Err(#ion::Error::new("Constructor must be called with \"new\".", ::std::option::Option::None).into());
			}

			wrapper(cx, args, &mut this)
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

pub(crate) fn error_handler(ion: &TokenStream, ty: &Type, return_type: &Type) -> TokenStream {
	let mut handler = quote!(
		#ion::functions::__handle_native_constructor_private_result(
			cx,
			result,
			<#ty as #ion::ClassDefinition>::PARENT_PROTOTYPE_CHAIN_LENGTH,
			&this,
			args.rval(),
		)
	);
	if return_type == &parse_quote!(()) {
		handler = quote!(#ion::functions::__handle_native_constructor_result(cx, result, &this, args.rval()));
	}
	if let Type::Path(ty) = &return_type {
		if type_ends_with(ty, "Result") || type_ends_with(ty, "ResultExc") {
			if let PathArguments::AngleBracketed(args) = &ty.path.segments.last().unwrap().arguments {
				if let Some(GenericArgument::Type(Type::Tuple(ty))) = args.args.first() {
					if ty.elems.is_empty() {
						handler = quote!(#ion::functions::__handle_native_constructor_result(cx, result, &this, args.rval()));
					}
				}
			}
		}
	}
	handler
}
