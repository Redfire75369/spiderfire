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

pub(crate) fn from_jsval(class_ident: &Ident) -> ItemImpl {
	let krate = quote!(::ion);

	parse2(quote!(
		impl ::mozjs::conversions::FromJSValConvertible for #class_ident {
			type Config = ();
			unsafe fn from_jsval(cx: #krate::Context, value: ::mozjs::rust::HandleValue, _: ()) -> ::std::result::Result<::mozjs::conversions::ConversionResult<#class_ident>, ()> {
				#krate::class::class_from_jsval(cx, value)
			}
		}
	)).unwrap()
}

pub(crate) fn to_jsval(class_ident: &Ident, is_clone: bool) -> ItemImpl {
	let krate = quote!(::ion);

	if is_clone {
		parse2(quote!(
			impl ::mozjs::conversions::ToJSValConvertible for #class_ident {
				unsafe fn to_jsval(&self, cx: #krate::Context, mut rval: ::mozjs::rust::MutableHandleValue) {
					rval.set(<#class_ident as #krate::ClassInitialiser>::new_object(cx, self.clone()).to_value());
				}
			}
		))
		.unwrap()
	} else {
		parse2(quote!(
			impl #krate::conversions::IntoJSVal for #class_ident {
				unsafe fn into_jsval(self: Box<Self>, cx: #krate::Context, mut rval: ::mozjs::rust::MutableHandleValue) {
					rval.set(<#class_ident as #krate::ClassInitialiser>::new_object(cx, *self).to_value());
				}
			}
		))
		.unwrap()
	}
}
