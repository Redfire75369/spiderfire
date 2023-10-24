/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use syn::{FnArg, GenericParam, ItemFn, parse2, PathArguments, Result, ReturnType, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::attribute::krate::Crates;
use crate::function::inner::impl_inner_fn;
use crate::function::parameters::Parameters;
use crate::utils::path_ends_with;

pub(crate) fn impl_wrapper_fn(
	crates: &Crates, mut function: ItemFn, class_ty: Option<&Type>, keep_inner: bool, is_constructor: bool,
) -> Result<(ItemFn, Parameters)> {
	if function.sig.asyncness.is_some() {
		return impl_async_wrapper_fn(crates, function, class_ty, keep_inner);
	}

	let ion = &crates.ion;
	let parameters = Parameters::parse(&function.sig.inputs, class_ty)?;
	let idents = parameters.to_idents();
	let statements = parameters.to_statements(ion)?;
	let mut this_statements = parameters.to_this_statement(ion, class_ty.is_some(), false)?;

	let inner = impl_inner_fn(function.clone(), &parameters, keep_inner)?;

	let argument_checker = argument_checker(ion, &function.sig.ident, parameters.nargs.0);

	let wrapper_generics: [GenericParam; 2] = [parse_quote!('cx), parse_quote!('a)];
	let mut wrapper_args: Vec<FnArg> = vec![
		parse_quote!(__cx: &'cx #ion::Context),
		parse_quote!(__args: &'a mut #ion::Arguments<'_, 'cx>),
	];
	if is_constructor {
		wrapper_args.push(parse_quote!(__this: &mut #ion::Object<'cx>));
	} else {
		this_statements = this_statements.map(|statement| {
			quote!(
				let __this = &mut __accessor.this_mut().to_object(__cx);
				#statement
			)
		});
	}

	let mut output = match &function.sig.output {
		ReturnType::Default => parse_quote!(()),
		ReturnType::Type(_, ty) => *ty.clone(),
	};
	let mut wrapper_output = output.clone();

	let mut result = quote!(__result.map_err(::std::convert::Into::into));

	if let Type::Path(ty) = &output {
		if !path_ends_with(&ty.path, "ResultExc") {
			if path_ends_with(&ty.path, "Result") {
				if let PathArguments::AngleBracketed(args) = &ty.path.segments.last().unwrap().arguments {
					let arg = args.args.first().unwrap();
					wrapper_output = parse_quote!(#ion::ResultExc<#arg>);
				}
			} else {
				result = quote!(#ion::ResultExc::<#ty>::Ok(__result));
				wrapper_output = parse_quote!(#ion::ResultExc::<#ty>);
			}
		}
	} else {
		result = quote!(#ion::ResultExc::<#output>::Ok(__result));
		wrapper_output = parse_quote!(#ion::ResultExc::<#output>);
	}

	if function.sig.asyncness.is_some() {
		result = quote!(__result);

		output = parse_quote!(#ion::ResultExc::<#ion::Promise<'cx>>);
		wrapper_output = parse_quote!(#ion::ResultExc::<#ion::Promise<'cx>>);
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

		let mut __accessor = __args.access();
		#this_statements
		#(#statements)*

		#wrapper_inner

		#[allow(clippy::let_unit_value)]
		let __result: #output = #inner_call;
		#result
	}))?;

	function.sig.ident = format_ident!("wrapper", span = function.sig.ident.span());
	function.sig.inputs = Punctuated::from_iter(wrapper_args);
	function.sig.generics.params = Punctuated::from_iter(wrapper_generics);
	function.sig.output = parse_quote!(-> #wrapper_output);
	function.sig.asyncness = None;
	function.sig.unsafety = Some(<Token![unsafe]>::default());

	function.attrs.clear();
	function.block = body;

	Ok((function, parameters))
}

pub(crate) fn impl_async_wrapper_fn(
	crates: &Crates, mut function: ItemFn, class_ty: Option<&Type>, keep_inner: bool,
) -> Result<(ItemFn, Parameters)> {
	let Crates { ion, runtime } = crates;

	let parameters = Parameters::parse(&function.sig.inputs, class_ty)?;
	let idents = &parameters.idents;
	let statements = parameters.to_statements(ion)?;
	let this_statement = parameters.to_this_statement(ion, class_ty.is_some(), true)?;

	let inner = impl_inner_fn(function.clone(), &parameters, keep_inner)?;

	let argument_checker = argument_checker(ion, &function.sig.ident, parameters.nargs.0);

	let wrapper_generics: [GenericParam; 2] = [parse_quote!('cx), parse_quote!('a)];
	let wrapper_args: Vec<FnArg> = vec![
		parse_quote!(__cx: &'cx #ion::Context),
		parse_quote!(__args: &'a mut #ion::Arguments<'_, 'cx>),
	];

	let inner_output = match &function.sig.output {
		ReturnType::Default => parse_quote!(()),
		ReturnType::Type(_, ty) => *ty.clone(),
	};
	let output = quote!(#ion::Promise<'cx>);
	let wrapper_output = quote!(#ion::ResultExc<#output>);

	let mut is_result = false;
	if let Type::Path(ty) = &inner_output {
		if path_ends_with(&ty.path, "Result") || path_ends_with(&ty.path, "ResultExc") {
			is_result = true;
		}
	}

	let async_result = if is_result {
		quote!(__result)
	} else {
		quote!(#ion::ResultExc::<#inner_output>::Ok(__result))
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
		quote!(
			let mut __this: #ion::Object<'static> = ::std::mem::transmute(#ion::Object::from(__cx.root_persistent_object(__accessor.this_mut().handle().to_object())));
			let __cx2: #ion::Context<'static> = #ion::Context::new_unchecked(__cx.as_ptr());
		)
	});
	let unrooter = parameters
		.this
		.is_some()
		.then(|| quote!(__cx2.unroot_persistent_object(__this.handle().get());));

	let body = parse2(quote_spanned!(function.span() => {
		#argument_checker

		let mut __accessor = __args.access();
		#(#statements)*
		#wrapper_inner

		#wrapper

		let __result: #output = {
			let __future = async move {
				#this_statement

				#[allow(clippy::let_unit_value)]
				let __result: #inner_output = #call.await;

				#unrooter
				#async_result
			};

			#runtime::promise::future_to_promise(__cx, __future)
		};
		::std::result::Result::Ok(__result)
	}))?;

	function.sig.ident = format_ident!("wrapper", span = function.sig.ident.span());
	function.sig.inputs = Punctuated::from_iter(wrapper_args);
	function.sig.generics.params = Punctuated::from_iter(wrapper_generics);
	function.sig.output = parse_quote!(-> #wrapper_output);
	function.sig.asyncness = None;
	function.sig.unsafety = Some(<Token![unsafe]>::default());

	function.block = body;

	Ok((function, parameters))
}

pub(crate) fn argument_checker(ion: &TokenStream, ident: &Ident, nargs: usize) -> TokenStream {
	if nargs != 0 {
		let plural = if nargs == 1 { "" } else { "s" };
		let error = format!("{}() requires at least {} argument{}", ident, nargs, plural);
		quote!(
			if __args.len() < #nargs {
				return ::std::result::Result::Err(#ion::Error::new(#error, ::std::option::Option::None).into());
			}
		)
	} else {
		TokenStream::new()
	}
}
