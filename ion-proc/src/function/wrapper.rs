/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use syn::{FnArg, ItemFn, LitStr, parse2, Result, ReturnType, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::function::inner::impl_inner_fn;
use crate::function::parameters::Parameters;
use crate::utils::type_ends_with;

pub(crate) fn impl_wrapper_fn(
	mut function: ItemFn, class: Option<&Ident>, keep_inner: bool, is_constructor: bool,
) -> Result<(ItemFn, ItemFn, Parameters)> {
	let krate = quote!(::ion);

	let parameters = Parameters::parse(&function.sig.inputs, class, class.is_some())?;
	let idents = &parameters.idents;
	let statements = parameters.to_statements(class.is_some())?;

	let inner = impl_inner_fn(function.clone(), &parameters, keep_inner)?;

	let argument_checker = argument_checker(&function.sig.ident, parameters.nargs.0);
	let constructing_checker = constructing_checker(is_constructor);

	let wrapper_args: [FnArg; 2] = [parse_quote!(cx: #krate::Context), parse_quote!(args: &#krate::Arguments)];

	let mut output = match &function.sig.output {
		ReturnType::Default => parse_quote!(()),
		ReturnType::Type(_, ty) => *ty.clone(),
	};
	let inner_output = output.clone();
	let mut wrapper_output = output.clone();

	let mut result = quote!(match result {
		::std::result::Result::Ok(o) => ::std::result::Result::Ok(o.into()),
		::std::result::Result::Err(e) => ::std::result::Result::Err(e.into()),
	});
	let mut async_result = result.clone();

	let mut is_result = false;
	if let Type::Path(ty) = &output {
		if !type_ends_with(ty, "Result") {
			result = quote!(::std::result::Result::<#ty, #krate::Exception>::Ok(result));
			wrapper_output = parse_quote!(::std::result::Result::<#ty, #krate::Exception>);
		} else {
			is_result = true;
		}
	} else {
		result = quote!(::std::result::Result::<#output, #krate::Exception>::Ok(result));
		wrapper_output = parse_quote!(::std::result::Result::<#output, #krate::Exception>);
	}

	if function.sig.asyncness.is_some() {
		result = quote!(result);
		if !is_result {
			async_result = quote!(::std::result::Result::<#inner_output, #krate::Exception>::Ok(result));
		} else {
			async_result = quote!(result);
		}

		output = parse_quote!(::std::result::Result::<#krate::Promise, #krate::Exception>);
		wrapper_output = parse_quote!(::std::result::Result::<#krate::Promise, #krate::Exception>);
	}

	let wrapper_inner = keep_inner.then_some(&inner);

	let mut call = quote!(inner);
	if !keep_inner {
		if let Some(class) = class {
			let function = &function.sig.ident;
			if parameters.this == Some(<Token![self]>::default().into()) {
				call = quote!(#function);
			} else {
				call = quote!(#class::#function);
			}
		}
	}

	let mut inner_call = if parameters.this == Some(<Token![self]>::default().into()) {
		quote!(self_.#call(#(#idents),*))
	} else {
		quote!(#call(#(#idents),*))
	};

	if function.sig.asyncness.is_some() {
		inner_call = quote!({
			let future = async move {
				#[allow(clippy::let_unit_value)]
				let result: #inner_output = #inner_call.await;
				#async_result
			};
			if let ::std::option::Option::Some(promise) = ::runtime::promise::future_to_promise(cx, future) {
				::std::result::Result::Ok(promise)
			} else {
				::std::result::Result::Err(#krate::Error::new("Failed to create Promise", None).into())
			}
		});
	}

	if is_constructor {
		let constructor_result = if !is_result {
			quote!(let result = ::std::result::Result::<#inner_output, ::ion::Exception>::Ok(result);)
		} else {
			TokenStream::new()
		};
		result = quote!(
			#constructor_result
			let result = result.map(|result| unsafe {
				let b = ::std::boxed::Box::new(result);
				::mozjs::rooted!(in(cx) let this = ::mozjs::jsapi::JS_NewObjectForConstructor(cx, &CLASS, &args.call_args()));
				::mozjs::jsapi::SetPrivate(this.get(), Box::into_raw(b) as *mut ::std::ffi::c_void);
				::mozjs::conversions::ToJSValConvertible::to_jsval(&this.get(), cx, ::mozjs::rust::MutableHandle::from_raw(args.rval()));
			});
			match result {
				::std::result::Result::Ok(_) => ::std::result::Result::Ok(()),
				::std::result::Result::Err(e) => ::std::result::Result::Err(e.into()),
			}
		);
		wrapper_output = parse_quote!(::std::result::Result::<(), #krate::Exception>);
	}

	let body = parse2(quote_spanned!(function.span() => {
		#argument_checker
		#constructing_checker
		#(#statements)*
		#wrapper_inner

		#[allow(clippy::let_unit_value)]
		let result: #output = #inner_call;
		#result
	}))?;

	function.sig.ident = Ident::new("wrapper", function.sig.ident.span());
	function.sig.inputs = Punctuated::from_iter(wrapper_args);
	function.sig.output = parse_quote!(-> #wrapper_output);
	function.sig.asyncness = None;
	function.sig.unsafety = Some(<Token![unsafe]>::default());

	function.block = body;

	Ok((function, inner, parameters))
}

pub(crate) fn argument_checker(ident: &Ident, nargs: usize) -> TokenStream {
	if nargs != 0 {
		let krate = quote!(::ion);

		let plural = if nargs == 1 { "" } else { "s" };
		let error_msg = format!("{}() requires at least {} argument{}", ident, nargs, plural);
		let error = LitStr::new(&error_msg, ident.span());
		quote!(
			if args.len() < #nargs {
				return ::std::result::Result::Err(#krate::Error::new(#error, ::std::option::Option::None).into());
			}
		)
	} else {
		TokenStream::new()
	}
}

pub(crate) fn constructing_checker(is_constructor: bool) -> Option<TokenStream> {
	let krate = quote!(::ion);

	if is_constructor {
		Some(quote!(
			if !args.is_constructing() {
				return ::std::result::Result::Err(#krate::Error::new("Constructor must be called with \"new\".", ::std::option::Option::None).into());
			}
		))
	} else {
		None
	}
}
