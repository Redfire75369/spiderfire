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
	let mut wrapper_output = output.clone();
	let mut result = quote!(match result {
		Ok(o) => Ok(o.into()),
		Err(e) => Err(e.into()),
	});
	if let Type::Path(ty) = &output {
		if !type_ends_with(ty, "Result") {
			result = quote!(::std::result::Result::<#ty, #krate::Exception>::Ok(result));
			wrapper_output = parse_quote!(::std::result::Result::<#ty, #krate::Exception>);
		}
	} else {
		result = quote!(::std::result::Result::<#output, #krate::Exception>::Ok(result));
		wrapper_output = parse_quote!(::std::result::Result::<#output, #krate::Exception>);
	}

	if function.sig.asyncness.is_some() {
		result = quote!(result);
		output = parse_quote!(::std::result::Result::<#krate::Promise, #krate::Exception>);
		wrapper_output = parse_quote!(::std::result::Result::<#krate::Promise, #krate::Exception>);
	}

	let wrapper_inner = keep_inner.then(|| &inner);

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
			let future = async move { #call(#(#idents),*).await };
			if let Some(promise) = ::runtime::promise::future_to_promise(cx, future) {
				Ok(promise)
			} else {
				Err(#krate::Error::new("Failed to create Promise", None).into())
			}
		});
	}

	if is_constructor {
		result = quote!(
			let result = result.map(|result| unsafe {
				let b = ::std::boxed::Box::new(result);
				::mozjs::rooted!(in(cx) let this = ::mozjs::jsapi::JS_NewObjectForConstructor(cx, &CLASS, &args.call_args()));
				::mozjs::jsapi::SetPrivate(this.get(), Box::into_raw(b) as *mut ::std::ffi::c_void);
				use ::mozjs::conversions::ToJSValConvertible;
				this.get().to_jsval(cx, ::mozjs::rust::MutableHandle::from_raw(args.rval()));
			});
			match result {
				Ok(_) => Ok(()),
				Err(e) => Err(e.into()),
			}
		);
		wrapper_output = parse_quote!(::std::result::Result::<(), #krate::Exception>);
	}

	let body = parse2(quote_spanned!(function.span() => {
		#argument_checker
		#constructing_checker
		#(#statements)*
		#wrapper_inner

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
				return Err(#krate::Error::new(#error, None).into());
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
				return Err(#krate::Error::new("Constructor must be called with \"new\".", None).into());
			}
		))
	} else {
		None
	}
}
