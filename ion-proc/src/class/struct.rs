/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::CString;

use proc_macro2::{Ident, Span, TokenStream};
use syn::{Error, Fields, ImplItemFn, ItemImpl, ItemStruct, LitStr, Member, parse2, Path, Result, Type};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::attribute::krate::crate_from_attributes;
use crate::utils::path_ends_with;

pub(super) fn impl_js_class_struct(r#struct: &mut ItemStruct) -> Result<[ItemImpl; 6]> {
	let ion = &crate_from_attributes(&r#struct.attrs);

	let repr_c = r#struct.attrs.iter().fold(Ok(false), |acc, attr| {
		if attr.path().is_ident("repr") {
			return match attr.parse_args::<Ident>() {
				Ok(ident) if ident == "C" => Ok(true),
				_ => Err(Error::new(attr.span(), "Only C Representations are allowed.")),
			};
		}
		acc
	})?;
	if !repr_c {
		r#struct.attrs.push(parse_quote!(#[repr(C)]));
	}

	let traceable = r#struct.attrs.iter().any(|attr| {
		if attr.path().is_ident("derive") {
			if let Ok(paths) = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated) {
				return paths.iter().any(|path| path_ends_with(path, "Traceable"));
			}
		}
		false
	});
	if !traceable {
		r#struct.attrs.push(parse_quote!(#[derive(#ion::Traceable)]));
	}

	if !r#struct.generics.params.is_empty() {
		return Err(Error::new(r#struct.generics.span(), "Native Class Structs cannot have generics."));
	}

	let ident = &r#struct.ident;
	let r#type: Type = parse2(quote_spanned!(ident.span() => #ident))?;
	let super_field;
	let super_type;

	let err = Err(Error::new(r#struct.span(), "Native Class Structs must have at least a reflector field."));
	match &r#struct.fields {
		Fields::Named(fields) => match fields.named.first() {
			Some(field) => {
				super_field = Member::Named(field.ident.as_ref().unwrap().clone());
				super_type = field.ty.clone();
			}
			None => return err,
		},
		Fields::Unnamed(fields) => match fields.unnamed.first() {
			Some(field) => {
				super_field = parse_quote!(0);
				super_type = field.ty.clone()
			}
			None => return err,
		},
		Fields::Unit => return err,
	}

	if let Type::Path(ty) = &super_type {
		if ty.path.segments.iter().any(|segment| !segment.arguments.is_empty()) {
			return Err(Error::new(super_type.span(), "Superclass Type must not have generics."));
		}
	} else {
		return Err(Error::new(super_type.span(), "Superclass Type must be a path."));
	}

	class_impls(ion, r#struct.span(), &ident.to_string(), &r#type, &super_field, &super_type)
}

fn class_impls(ion: &TokenStream, span: Span, name: &str, r#type: &Type, super_field: &Member, super_type: &Type) -> Result<[ItemImpl; 6]> {
	let from_value = impl_from_value(ion, span, name, r#type, false)?;
	let from_value_mut = impl_from_value(ion, span, name, r#type, true)?;

	let derived_from = parse2(quote_spanned!(span => unsafe impl #ion::class::DerivedFrom<#super_type> for #r#type {}))?;
	let castable = parse2(quote_spanned!(span => impl #ion::class::Castable for #r#type {}))?;

	let native_object = parse2(quote_spanned!(span => impl #ion::class::NativeObject for #r#type {
		fn reflector(&self) -> &#ion::class::Reflector {
			#ion::class::NativeObject::reflector(&self.#super_field)
		}
	}))?;

	let none = quote!(::std::option::Option::None);

	let operations = class_operations(span)?;
	let name = String::from_utf8(CString::new(name).unwrap().into_bytes_with_nul()).unwrap();

	let mut operations_native_class: ItemImpl = parse2(quote_spanned!(span => impl #r#type {
		#(#operations)*

		pub const fn __ion_native_prototype_chain() -> #ion::class::PrototypeChain {
			const ION_TYPE_ID: #ion::class::TypeIdWrapper<#r#type> = #ion::class::TypeIdWrapper::new();

			let mut proto_chain = #super_type::__ion_native_prototype_chain();
			let mut i = 0;
			while i < #ion::class::MAX_PROTO_CHAIN_LENGTH {
				match proto_chain[i] {
					Some(_) => i += 1,
					None => {
						proto_chain[i] = Some(&ION_TYPE_ID);
						break;
					}
				}
			}
			proto_chain
		}

		pub const fn __ion_native_class() -> &'static #ion::class::NativeClass {
			const ION_CLASS_OPERATIONS: ::mozjs::jsapi::JSClassOps = ::mozjs::jsapi::JSClassOps {
				addProperty: #none,
				delProperty: #none,
				enumerate: #none,
				newEnumerate: #none,
				resolve: #none,
				mayResolve: #none,
				finalize: ::std::option::Option::Some(#r#type::__ion_finalise_operation),
				call: #none,
				construct: #none,
				trace: ::std::option::Option::Some(#r#type::__ion_trace_operation),
			};

			const ION_NATIVE_CLASS: #ion::class::NativeClass = #ion::class::NativeClass {
				base: ::mozjs::jsapi::JSClass {
					name: #name.as_ptr().cast(),
					flags: #ion::objects::class_reserved_slots(1) | ::mozjs::jsapi::JSCLASS_BACKGROUND_FINALIZE,
					cOps: &ION_CLASS_OPERATIONS as *const _,
					spec: ::std::ptr::null_mut(),
					ext: ::std::ptr::null_mut(),
					oOps: ::std::ptr::null_mut(),
				},
				prototype_chain: #r#type::__ion_native_prototype_chain(),
			};

			&ION_NATIVE_CLASS
		}
	}))?;
	operations_native_class.attrs.push(parse_quote!(#[doc(hidden)]));

	Ok([from_value, from_value_mut, derived_from, castable, native_object, operations_native_class])
}

fn impl_from_value(ion: &TokenStream, span: Span, name: &str, r#type: &Type, mutable: bool) -> Result<ItemImpl> {
	let from_value_error = LitStr::new(&format!("Expected {}", name), span);
	let function = if mutable { quote!(get_mut_private) } else { quote!(get_private) };
	let mutable = mutable.then(<Token![mut]>::default);

	parse2(
		quote_spanned!(span => impl<'cx> #ion::conversions::FromValue<'cx> for &'cx #mutable #r#type {
			type Config = ();

			fn from_value(cx: &'cx #ion::Context, value: &#ion::Value, strict: ::core::primitive::bool, _: ()) -> #ion::Result<&'cx #mutable #r#type> {
				let #mutable object = #ion::Object::from_value(cx, value, strict, ())?;
				if <#r#type as #ion::class::ClassDefinition>::instance_of(cx, &object, None) {
					Ok(<#r#type as #ion::class::ClassDefinition>::#function(&#mutable object))
				} else {
					Err(#ion::Error::new(#from_value_error, #ion::ErrorKind::Type))
				}
			}
		}),
	)
}

fn class_operations(span: Span) -> Result<Vec<ImplItemFn>> {
	let finalise = parse2(
		quote_spanned!(span => unsafe extern "C" fn __ion_finalise_operation(_: *mut ::mozjs::jsapi::GCContext, this: *mut ::mozjs::jsapi::JSObject) {
				let mut value = ::mozjs::jsval::NullValue();
				unsafe {
					::mozjs::glue::JS_GetReservedSlot(this, 0, &mut value);
				}
				if value.is_double() && value.asBits_ & 0xFFFF000000000000 == 0 {
					let private = value.to_private().cast_mut() as *mut Self;
					let _ = unsafe { ::std::boxed::Box::from_raw(private) };
				}
			}
		),
	)?;

	let trace = parse2(
		quote_spanned!(span => unsafe extern "C" fn __ion_trace_operation(trc: *mut ::mozjs::jsapi::JSTracer, this: *mut ::mozjs::jsapi::JSObject) {
				let mut value = ::mozjs::jsval::NullValue();
				unsafe {
					::mozjs::glue::JS_GetReservedSlot(this, 0, &mut value);
				}
				if value.is_double() && value.asBits_ & 0xFFFF000000000000 == 0 {
					unsafe {
						let private = &*(value.to_private() as *const Self);
						::mozjs::gc::Traceable::trace(private, trc);
					}
				}
			}
		),
	)?;

	Ok(vec![finalise, trace])
}
