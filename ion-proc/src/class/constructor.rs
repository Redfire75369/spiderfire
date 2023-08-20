/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{GenericArgument, ItemFn, PathArguments, Result, ReturnType, Type};

use crate::class::method::{Method, MethodReceiver};
use crate::function::{check_abi, set_signature};
use crate::function::parameters::Parameters;
use crate::function::wrapper::impl_wrapper_fn;
use crate::utils::type_ends_with;

pub(crate) fn impl_constructor(mut constructor: ItemFn, ty: &Type) -> Result<(Method, Parameters)> {
	let krate = quote!(::ion);
	let (wrapper, inner, parameters) = impl_wrapper_fn(constructor.clone(), Some(ty), false, true)?;

	check_abi(&mut constructor)?;
	set_signature(&mut constructor)?;
	constructor.attrs.clear();
	constructor.attrs.push(parse_quote!(#[allow(non_snake_case)]));

	let empty = Box::new(parse_quote!(()));
	let return_type = match &wrapper.sig.output {
		ReturnType::Default => &empty,
		ReturnType::Type(_, ty) => ty,
	};
	let error_handler = error_handler(ty, return_type);

	let body = parse_quote!({
		let cx = &#krate::Context::new_unchecked(cx);
		let mut args = #krate::Arguments::new(cx, argc, vp);
		let mut this = #krate::Object::from(cx.root_object(::mozjs::jsapi::JS_NewObjectForConstructor(cx.as_ptr(), &CLASS, &args.call_args())));

		#wrapper
		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
			if !args.is_constructing() {
				return ::std::result::Result::Err(#krate::Error::new("Constructor must be called with \"new\".", ::std::option::Option::None).into());
			}

			wrapper(cx, &mut args, &mut this)
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

pub(crate) fn error_handler(ty: &Type, return_type: &Type) -> TokenStream {
	let krate = quote!(::ion);
	let mut if_ok = quote!(
		let b = ::std::boxed::Box::new(::std::option::Option::Some(value));
		::mozjs::jsapi::JS_SetReservedSlot(**this, <#ty as #krate::ClassInitialiser>::PARENT_PROTOTYPE_CHAIN_LENGTH, &::mozjs::jsval::PrivateValue(Box::into_raw(b) as *mut ::std::ffi::c_void));
		#krate::conversions::ToValue::to_value(&this, cx, args.rval());
		true
	);
	if return_type == &parse_quote!(()) {
		if_ok = quote!(
			#krate::conversions::ToValue::to_value(&this, cx, args.rval());
			true
		);
	}
	if let Type::Path(ty) = &return_type {
		if type_ends_with(ty, "Result") || type_ends_with(ty, "ResultExc") {
			if let PathArguments::AngleBracketed(args) = &ty.path.segments.last().unwrap().arguments {
				if let Some(GenericArgument::Type(Type::Tuple(ty))) = args.args.first() {
					if ty.elems.is_empty() {
						if_ok = quote!(
							#krate::conversions::ToValue::to_value(&this, cx, args.rval());
							true
						);
					}
				}
			}
		}
	}
	quote!({
		use ::std::prelude::v1::*;

		match result {
			Ok(Ok(value)) => {
				#if_ok
			},
			Ok(Err(error)) => {
				#krate::exception::ThrowException::throw(&error, cx);
				false
			}
			Err(unwind_error) => {
				if let Some(unwind) = unwind_error.downcast_ref::<String>() {
					#krate::exception::ThrowException::throw(&#krate::Error::new(unwind, ::std::option::Option::None), cx);
				} else if let Some(unwind) = unwind_error.downcast_ref::<&str>() {
					#krate::exception::ThrowException::throw(&#krate::Error::new(*unwind, ::std::option::Option::None), cx);
				} else {
					#krate::exception::ThrowException::throw(&#krate::Error::new("Unknown Panic Occurred", None), cx);
					::std::mem::forget(unwind_error);
				}
				false
			}
		}
	})
}
