/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use syn::{Block, ItemFn, Result};

use crate::function::{check_abi, set_signature};
use crate::function::inner::{DefaultInnerBody, impl_inner_fn, InnerBody};

pub(crate) fn impl_constructor(mut constructor: ItemFn) -> Result<(ItemFn, usize)> {
	let krate = quote!(::ion);
	let (mut inner, nargs, _) = impl_inner_fn::<ClassConstructorInnerBody>(&constructor, true)?;

	inner.sig.output = parse_quote!(-> #krate::Result<()>);

	check_abi(&mut constructor)?;
	set_signature(&mut constructor)?;
	constructor.attrs.push(parse_quote!(#[allow(non_snake_case)]));

	let error_handler = error_handler();

	let body = parse_quote!({
		let args = #krate::Arguments::new(argc, vp);

		if !args.is_constructing() {
			#krate::Error::Error(String::from("Constructor must be called with \"new\".")).throw(cx);
			return false;
		}

		#inner

		let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| native_fn(cx, &args)));

		#error_handler
	});
	constructor.block = Box::new(body);

	Ok((constructor, nargs))
}

pub(crate) struct ClassConstructorInnerBody;

impl InnerBody for ClassConstructorInnerBody {
	fn impl_inner(body: Box<Block>, this: Option<Ident>, is_async: bool) -> TokenStream {
		let body = DefaultInnerBody::impl_inner(body, this.clone(), is_async);
		quote!(
			let result = #body;

			use ::mozjs::conversions::ToJSValConvertible;

			result.map(|result| unsafe {
				let b = ::std::boxed::Box::new(result);
				::mozjs::rooted!(in(cx) let this = ::mozjs::jsapi::JS_NewObjectForConstructor(cx, &CLASS, &args.call_args()));
				::mozjs::jsapi::SetPrivate(this.get(), Box::leak(b) as *mut _ as *mut ::std::ffi::c_void);
				this.get().to_jsval(cx, ::mozjs::rust::MutableHandle::from_raw(args.rval()));
			})
		)
	}
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
				error.throw(cx);
				false
			}
			Err(unwind_error) => {
				if let Some(unwind) = unwind_error.downcast_ref::<String>() {
					#krate::Error::Error(unwind.clone()).throw(cx);
				} else if let Some(unwind) = unwind_error.downcast_ref::<&str>() {
					#krate::Error::Error(String::from(*unwind)).throw(cx);
				} else {
					#krate::Error::Error(String::from("Unknown Panic Occurred")).throw(cx);
					::std::mem::forget(unwind_error);
				}
				false
			}
		}
	})
}
