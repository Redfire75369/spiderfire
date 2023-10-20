/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use syn::{ItemImpl, parse2, Type};

use crate::attribute::krate::Crates;
use crate::class::constructor::impl_constructor;
use crate::class::method::Method;

pub(crate) fn no_constructor(crates: &Crates, ty: &Type) -> Method {
	let ion = &crates.ion;
	let method = parse2(quote!(
		pub fn constructor() -> #ion::Result<#ty> {
			#ion::Result::Err(#ion::Error::new("Constructor should not be called.", ::std::option::Option::None))
		}
	))
	.unwrap();

	impl_constructor(crates, method, ty).unwrap().0
}

pub(crate) fn from_value(ion: &TokenStream, class_ident: &Ident) -> ItemImpl {
	parse2(quote!(
		impl<'cx> #ion::conversions::FromValue<'cx> for #class_ident {
			type Config = ();
			fn from_value<'v>(cx: &'cx #ion::Context, value: &#ion::Value<'v>, _: bool, _: ()) -> #ion::Result<#class_ident> {
				#ion::class::class_from_value(cx, value)
			}
		}
	))
	.unwrap()
}

pub(crate) fn to_value(ion: &TokenStream, class_ident: &Ident, is_clone: bool) -> ItemImpl {
	if is_clone {
		parse2(quote!(
			impl<'cx> #ion::conversions::ToValue<'cx> for #class_ident {
				fn to_value(&self, cx: &'cx #ion::Context, value: &mut #ion::Value) {
					<#class_ident as #ion::ClassDefinition>::new_object(cx, self.clone()).to_value(cx, value)
				}
			}
		))
		.unwrap()
	} else {
		parse2(quote!(
			impl<'cx> #ion::conversions::IntoValue<'cx> for #class_ident {
				fn into_value(self: ::std::boxed::Box<Self>, cx: &'cx #ion::Context, value: &mut #ion::Value) {
					#ion::conversions::ToValue::to_value(&<#class_ident as #ion::ClassDefinition>::new_object(cx, *self), cx, value)
				}
			}
		))
		.unwrap()
	}
}
