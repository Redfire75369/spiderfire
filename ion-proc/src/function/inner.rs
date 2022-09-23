/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use syn::{ItemFn, Result};
use syn::punctuated::Punctuated;

use crate::function::parameters::Parameters;

pub(crate) fn impl_inner_fn(mut function: ItemFn, parameters: &Parameters, keep_inner: bool) -> Result<ItemFn> {
	let arguments = parameters.to_args();

	if keep_inner {
		function.sig.ident = Ident::new("inner", function.sig.ident.span());
	}
	function.sig.inputs = Punctuated::from_iter(arguments);
	Ok(function)
}
