/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use syn::{ItemFn, Result};

use crate::function::{check_abi, error_handler, set_signature};
use crate::function::inner::{DefaultInnerBody, impl_inner_fn};

pub(crate) fn impl_method<F: FnOnce(usize) -> Result<()>>(mut method: ItemFn, predicate: F) -> Result<(ItemFn, usize, Option<Ident>)> {
	let krate = quote!(::ion);
	let (inner, nargs, this) = impl_inner_fn::<DefaultInnerBody>(&method, true)?;

	predicate(nargs).and_then(|_| {
		check_abi(&mut method)?;
		set_signature(&mut method)?;
		method.attrs.push(parse_quote!(#[allow(non_snake_case)]));

		let error_handler = error_handler();

		let body = parse_quote!({
			let args = #krate::Arguments::new(argc, vp);

			#inner

			let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| native_fn(cx, &args)));

			#error_handler
		});
		method.block = Box::new(body);

		Ok((method, nargs, this))
	})
}
