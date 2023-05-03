/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use syn::{FnArg, GenericParam, ItemFn, parse2, Result, ReturnType, Type, WhereClause};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::function::inner::impl_inner_fn;
use crate::function::parameters::Parameters;
use crate::utils::type_ends_with;

pub(crate) fn impl_wrapper_fn(mut function: ItemFn, class_ty: Option<&Type>, keep_inner: bool) -> Result<(ItemFn, ItemFn, Parameters)> {
	let krate = quote!(::ion);

	if function.sig.asyncness.is_some() {
		return impl_async_wrapper_fn(function, class_ty, keep_inner);
	}

	let parameters = Parameters::parse(&function.sig.inputs, class_ty)?;
	let idents = &parameters.idents;
	let statements = parameters.to_statements()?;
	let this_statements = parameters.to_this_statements(class_ty.is_some(), false)?;

	let inner = impl_inner_fn(function.clone(), &parameters, keep_inner)?;

	let argument_checker = argument_checker(&function.sig.ident, parameters.nargs.0);

	let wrapper_generics: [GenericParam; 2] = [parse_quote!('cx), parse_quote!('a)];
	let wrapper_where: WhereClause = parse_quote!(where 'cx: 'a);
	let wrapper_args: [FnArg; 2] = [parse_quote!(cx: &'cx #krate::Context), parse_quote!(args: &'a mut #krate::Arguments<'cx>)];

	let mut output = match &function.sig.output {
		ReturnType::Default => parse_quote!(()),
		ReturnType::Type(_, ty) => *ty.clone(),
	};
	let mut wrapper_output = output.clone();

	let mut result = quote!(result.map_err(::std::convert::Into::into));

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

		output = parse_quote!(::std::result::Result::<#krate::Promise<'cx>, #krate::Exception>);
		wrapper_output = parse_quote!(::std::result::Result::<#krate::Promise<'cx>, #krate::Exception>);
	}

	let wrapper_inner = keep_inner.then_some(&inner);

	let mut call = quote!(inner);
	if !keep_inner {
		if let Some(class) = class_ty {
			let function = &function.sig.ident;
			if parameters.get_this_ident() == Some(<Token![self]>::default().into()) {
				call = quote!(#function);
			} else {
				call = quote!(<#class>::#function);
			}
		}
	}

	let inner_call = if parameters.get_this_ident() == Some(<Token![self]>::default().into()) {
		quote!(self_.#call(#(#idents),*))
	} else {
		quote!(#call(#(#idents),*))
	};

	let body = parse2(quote_spanned!(function.span() => {
		#argument_checker

		#this_statements
		#(#statements)*

		#wrapper_inner

		#[allow(clippy::let_unit_value)]
		let result: #output = #inner_call;
		#result
	}))?;

	function.sig.ident = Ident::new("wrapper", function.sig.ident.span());
	function.sig.inputs = Punctuated::from_iter(wrapper_args);
	function.sig.generics.params = Punctuated::from_iter(wrapper_generics);
	function.sig.generics.where_clause = Some(wrapper_where);
	function.sig.output = parse_quote!(-> #wrapper_output);
	function.sig.asyncness = None;
	function.sig.unsafety = Some(<Token![unsafe]>::default());

	function.block = body;

	Ok((function, inner, parameters))
}

pub(crate) fn impl_async_wrapper_fn(mut function: ItemFn, class_ty: Option<&Type>, keep_inner: bool) -> Result<(ItemFn, ItemFn, Parameters)> {
	let krate = quote!(::ion);

	let parameters = Parameters::parse(&function.sig.inputs, class_ty)?;
	let idents = &parameters.idents;
	let statements = parameters.to_statements()?;
	let this_statements = parameters.to_this_statements(class_ty.is_some(), true)?;

	let inner = impl_inner_fn(function.clone(), &parameters, keep_inner)?;

	let argument_checker = argument_checker(&function.sig.ident, parameters.nargs.0);

	let wrapper_generics: [GenericParam; 2] = [parse_quote!('cx), parse_quote!('a)];
	let wrapper_where: WhereClause = parse_quote!(where 'cx: 'a);
	let wrapper_args: [FnArg; 2] = [parse_quote!(cx: &'cx #krate::Context), parse_quote!(args: &'a mut #krate::Arguments<'cx>)];

	let inner_output = match &function.sig.output {
		ReturnType::Default => parse_quote!(()),
		ReturnType::Type(_, ty) => *ty.clone(),
	};
	let output = quote!(::std::result::Result::<#krate::Promise<'cx>, #krate::Exception>);

	let mut is_result = false;
	if let Type::Path(ty) = &inner_output {
		if type_ends_with(ty, "Result") {
			is_result = true;
		}
	}

	let async_result = if is_result {
		quote!(result)
	} else {
		quote!(::std::result::Result::<#inner_output, #krate::Exception>::Ok(result))
	};

	let wrapper_inner = keep_inner.then_some(&inner);

	let mut call = quote!(inner);
	if !keep_inner {
		if let Some(class) = class_ty {
			let function = &function.sig.ident;
			if parameters.get_this_ident() == Some(<Token![self]>::default().into()) {
				call = quote!(#function);
			} else {
				call = quote!(<#class>::#function);
			}
		}
	}

	let call = if parameters.get_this_ident() == Some(<Token![self]>::default().into()) {
		quote!(self_.#call(#(#idents),*))
	} else {
		quote!(#call(#(#idents),*))
	};

	let wrapper = parameters.this.is_some().then(|| {
		quote!(let mut this: ::std::option::Option<#krate::utils::SendWrapper<#krate::Local<'static, *mut ::mozjs::jsapi::JSObject>>>
			= ::std::option::Option::Some(#krate::utils::SendWrapper::new(#krate::Context::root_persistent_object(**args.this().to_object(cx))));)
	});
	let unrooter = parameters
		.this
		.is_some()
		.then(|| quote!(#krate::Context::unroot_persistent_object(*this.take().unwrap().take());));

	let body = parse2(quote_spanned!(function.span() => {
		#argument_checker
		#(#statements)*
		#wrapper_inner

		#wrapper

		let result: #output = {
			let future = async move {
				#this_statements

				#[allow(clippy::let_unit_value)]
				let result: #inner_output = #call.await;

				#unrooter
				#async_result
			};

			if let ::std::option::Option::Some(promise) = ::runtime::promise::future_to_promise(cx, future) {
				::std::result::Result::Ok(promise)
			} else {
				::std::result::Result::Err(#krate::Error::new("Failed to create Promise", None).into())
			}
		};
		result
	}))?;

	function.sig.ident = Ident::new("wrapper", function.sig.ident.span());
	function.sig.inputs = Punctuated::from_iter(wrapper_args);
	function.sig.generics.params = Punctuated::from_iter(wrapper_generics);
	function.sig.generics.where_clause = Some(wrapper_where);
	function.sig.output = parse_quote!(-> #output);
	function.sig.asyncness = None;
	function.sig.unsafety = Some(<Token![unsafe]>::default());

	function.block = body;

	Ok((function, inner, parameters))
}

pub(crate) fn argument_checker(ident: &Ident, nargs: usize) -> TokenStream {
	if nargs != 0 {
		let krate = quote!(::ion);

		let plural = if nargs == 1 { "" } else { "s" };
		let error = format!("{}() requires at least {} argument{}", ident, nargs, plural);
		quote!(
			if args.len() < #nargs {
				return ::std::result::Result::Err(#krate::Error::new(#error, ::std::option::Option::None).into());
			}
		)
	} else {
		TokenStream::new()
	}
}
