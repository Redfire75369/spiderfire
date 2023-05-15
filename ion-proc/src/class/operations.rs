/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use syn::{ItemFn, ItemStatic};

pub(crate) fn class_finalise(class: &Ident) -> ItemFn {
	let krate = quote!(::ion);
	parse_quote!(
		unsafe extern "C" fn finalise_operation(_: *mut ::mozjs::jsapi::GCContext, this: *mut ::mozjs::jsapi::JSObject) {
			let mut value = ::mozjs::jsval::NullValue();
			::mozjs::glue::JS_GetReservedSlot(this, <#class as #krate::class::ClassInitialiser>::PARENT_PROTOTYPE_CHAIN_LENGTH, &mut value);
			if value.is_double() && value.asBits_ & 0xFFFF000000000000 == 0 {
				let private = &mut *(value.to_private() as *mut Option<#class>);
				let _ = private.take();
			}
		}
	)
}

pub(crate) fn class_trace(class: &Ident) -> ItemFn {
	let krate = quote!(::ion);
	parse_quote!(
		unsafe extern "C" fn trace_operation(trc: *mut ::mozjs::jsapi::JSTracer, this: *mut ::mozjs::jsapi::JSObject) {
			let mut value = ::mozjs::jsval::NullValue();
			::mozjs::glue::JS_GetReservedSlot(this, <#class as #krate::class::ClassInitialiser>::PARENT_PROTOTYPE_CHAIN_LENGTH, &mut value);
			if value.is_double() && value.asBits_ & 0xFFFF000000000000 == 0 {
				let private = &*(value.to_private() as *mut Option<#class>);
				::mozjs::gc::Traceable::trace(private, trc);
			}
		}
	)
}

pub(crate) fn class_ops(has_trace: bool) -> ItemStatic {
	let none = quote!(::std::option::Option::None);
	let trace = if has_trace {
		quote!(::std::option::Option::Some(trace_operation))
	} else {
		none.clone()
	};
	parse_quote!(
		static OPERATIONS: ::mozjs::jsapi::JSClassOps = ::mozjs::jsapi::JSClassOps {
			addProperty: #none,
			delProperty: #none,
			enumerate: #none,
			newEnumerate: #none,
			resolve: #none,
			mayResolve: #none,
			finalize: ::std::option::Option::Some(finalise_operation),
			call: #none,
			construct: #none,
			trace: #trace,
		};
	)
}
