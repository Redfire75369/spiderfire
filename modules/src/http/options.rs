/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use bytes::Bytes;
use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{ESClass, GetBuiltinClass, Unbox};
use mozjs::jsval::JSVal;

use ion::{Context, Object};

use crate::http::client::ClientRequestOptions;
use crate::http::header::HeadersInit;

#[derive(Derivative, FromJSVal)]
#[derivative(Default)]
pub(crate) struct RequestOptions {
	pub(crate) auth: Option<String>,
	#[derivative(Default(value = "true"))]
	#[ion(default = true)]
	pub(crate) set_host: bool,

	#[ion(default)]
	pub(crate) client: ClientRequestOptions,
	#[ion(default)]
	pub(crate) headers: HeadersInit,
	#[ion(parser = |b| parse_body(cx, b))]
	pub(crate) body: Option<Bytes>,
}

macro_rules! typedarray_to_bytes {
	($body:expr) => {
		Bytes::default()
	};
	($body:expr, [$arr:ident, true]$(, $($rest:tt)*)?) => {
		paste! {
			if let Ok(arr) = <::mozjs::typedarray::$arr>::from($body) {
				Bytes::copy_from_slice(arr.as_slice())
			} else if let Ok(arr) = <::mozjs::typedarray::[<Heap $arr>]>::from($body) {
				Bytes::copy_from_slice(arr.as_slice())
			} else {
				typedarray_to_bytes!($body$(, $($rest)*)?)
			}
		}
	};
	($body:expr, [$arr:ident, false]$(, $($rest:tt)*)?) => {
		paste! {
			if let Ok(arr) = <::mozjs::typedarray::$arr>::from($body) {
				let bytes: &[u8] = cast_slice(arr.as_slice());
				Bytes::copy_from_slice(bytes)
			} else if let Ok(arr) = <::mozjs::typedarray::[<Heap $arr>]>::from($body) {
				let bytes: &[u8] = cast_slice(arr.as_slice());
				Bytes::copy_from_slice(bytes)
			} else {
				typedarray_to_bytes!($body$(, $($rest)*)?)
			}
		}
	};
}

pub(crate) unsafe fn parse_body(cx: Context, body: JSVal) -> Bytes {
	if body.is_string() {
		Bytes::from(jsstr_to_string(cx, body.to_string()))
	} else if body.is_object() {
		let body = body.to_object();
		rooted!(in(cx) let rbody = body);

		let mut class = ESClass::Other;
		if GetBuiltinClass(cx, rbody.handle().into(), &mut class) {
			if class == ESClass::String {
				rooted!(in(cx) let mut unboxed = Object::new(cx).to_value());

				if Unbox(cx, rbody.handle().into(), unboxed.handle_mut().into()) {
					return Bytes::from(jsstr_to_string(cx, unboxed.get().to_string()));
				}
			}
		} else {
			return Bytes::default();
		}

		typedarray_to_bytes!(body, [ArrayBuffer, true], [ArrayBufferView, true])
	} else {
		Bytes::default()
	}
}
