/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use syn::{ItemFn, Result, Signature, Type};

use crate::attribute::class::Name;
use crate::function::{check_abi, impl_fn_body, set_signature};
use crate::function::parameters::Parameters;
use crate::function::wrapper::impl_wrapper_fn;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum MethodKind {
	Constructor,
	Getter,
	Setter,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum MethodReceiver {
	Dynamic,
	Static,
}

#[derive(Clone)]
pub(super) struct Method {
	pub(super) receiver: MethodReceiver,
	pub(super) method: ItemFn,
	pub(super) nargs: usize,
	pub(super) names: Vec<Name>,
}

pub(super) fn impl_method<F>(
	ion: &TokenStream, mut method: ItemFn, ty: &Type, predicate: F,
) -> Result<(Method, Parameters)>
where
	F: FnOnce(&Signature) -> Result<()>,
{
	let (wrapper, parameters) = impl_wrapper_fn(ion, method.clone(), Some(ty), false)?;

	predicate(&method.sig).and_then(|_| {
		check_abi(&mut method)?;
		set_signature(&mut method)?;

		method.attrs.clear();
		method.attrs.push(parse_quote!(#[allow(non_snake_case)]));
		method.sig.ident = format_ident!("__ion_bindings_method_{}", method.sig.ident);
		method.block = Box::new(impl_fn_body(ion, &wrapper)?);

		let method = Method {
			receiver: if parameters.this.is_some() {
				MethodReceiver::Dynamic
			} else {
				MethodReceiver::Static
			},
			method,
			nargs: parameters.nargs.0,
			names: vec![],
		};
		Ok((method, parameters))
	})
}
