/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::{ItemFn, Result, Signature, Type};

use crate::attribute::class::Name;
use crate::attribute::krate::Crates;
use crate::function::{check_abi, set_signature};
use crate::function::parameters::Parameters;
use crate::function::wrapper::impl_wrapper_fn;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum MethodKind {
	Constructor,
	Getter,
	Setter,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum MethodReceiver {
	Dynamic,
	Static,
}

#[derive(Clone, Debug)]
pub(crate) struct Method {
	pub(crate) receiver: MethodReceiver,
	pub(crate) method: ItemFn,
	pub(crate) nargs: usize,
	pub(crate) names: Vec<Name>,
}

pub(crate) fn impl_method<F>(crates: &Crates, mut method: ItemFn, ty: &Type, predicate: F) -> Result<(Method, Parameters)>
where
	F: FnOnce(&Signature) -> Result<()>,
{
	let (wrapper, parameters) = impl_wrapper_fn(crates, method.clone(), Some(ty), false, false)?;
	let ion = &crates.ion;

	predicate(&method.sig).and_then(|_| {
		check_abi(&mut method)?;
		set_signature(&mut method)?;
		method.attrs.clear();
		method.attrs.push(parse_quote!(#[allow(non_snake_case)]));

		let body = parse_quote!({
			let cx = &#ion::Context::new_unchecked(cx);
			let args = &mut #ion::Arguments::new(cx, argc, vp);

			#wrapper

			let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| wrapper(cx, args)));
			#ion::functions::__handle_native_function_result(cx, result, args.rval())
		});
		method.block = Box::new(body);
		method.sig.ident = format_ident!("__ion_bindings_method_{}", method.sig.ident);

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
