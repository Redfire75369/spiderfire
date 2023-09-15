/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use syn::{ItemImpl, parse2, Type};

use crate::class::constructor::impl_constructor;
use crate::class::method::Method;

pub(crate) fn no_constructor(ty: &Type) -> Method {
	let krate = quote!(::ion);

	let method = parse2(quote!(
		pub fn constructor() -> #krate::Result<#ty> {
			#krate::Result::Err(#krate::Error::new("Constructor should not be called.", ::std::option::Option::None))
		}
	))
	.unwrap();

	impl_constructor(method, ty).unwrap().0
}

pub(crate) fn from_value(class_ident: &Ident) -> ItemImpl {
	let krate = quote!(::ion);

	parse2(quote!(
		impl<'cx> ::ion::conversions::FromValue<'cx> for #class_ident {
			type Config = ();
			unsafe fn from_value<'v>(cx: &'cx #krate::Context, value: &#krate::Value<'v>, _: bool, _: ()) -> #krate::Result<#class_ident>
			where
				'cx: 'v
			{
				#krate::class::class_from_value(cx, value)
			}
		}
	))
	.unwrap()
}

pub(crate) fn to_value(class_ident: &Ident, is_clone: bool) -> ItemImpl {
	let krate = quote!(::ion);

	if is_clone {
		parse2(quote!(
			impl<'cx> #krate::conversions::ToValue<'cx> for #class_ident {
				unsafe fn to_value(&self, cx: &'cx #krate::Context, value: &mut #krate::Value) {
					<#class_ident as #krate::ClassInitialiser>::new_object(cx, self.clone()).to_value(cx, value)
				}
			}
		))
		.unwrap()
	} else {
		parse2(quote!(
			impl<'cx> #krate::conversions::IntoValue<'cx> for #class_ident {
				unsafe fn into_value(self: ::std::boxed::Box<Self>, cx: &'cx #krate::Context, value: &mut #krate::Value) {
					#krate::conversions::ToValue::to_value(&<#class_ident as #krate::ClassInitialiser>::new_object(cx, *self), cx, value)
				}
			}
		))
		.unwrap()
	}
}
