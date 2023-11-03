/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use base64::{decoded_len_estimate, Engine};
use base64::prelude::BASE64_STANDARD;
use data_url::forgiving_base64::{DecodeError, Decoder};
use mozjs::jsapi::JSFunctionSpec;

use ion::{Context, Error, ErrorKind, Object, Result, StringRef};

const INVALID_CHARACTER_EXCEPTION: &str = "String contains an invalid character.";

#[js_fn]
fn btoa(data: StringRef) -> Result<String> {
	match data {
		StringRef::Latin1(bytes) => Ok(BASE64_STANDARD.encode(bytes)),
		StringRef::Utf16(wstr) => {
			let bytes = wstr
				.as_bytes()
				.chunks_exact(2)
				.map(|chunk| {
					let codepoint = u16::from_ne_bytes([chunk[0], chunk[1]]);
					if codepoint > u8::MAX as u16 {
						Err(Error::new(INVALID_CHARACTER_EXCEPTION, ErrorKind::Range))
					} else {
						Ok(codepoint as u8)
					}
				})
				.collect::<Result<Vec<_>>>()?;
			Ok(BASE64_STANDARD.encode(bytes))
		}
	}
}

#[js_fn]
fn atob(data: StringRef) -> Result<String> {
	fn decode_err(_: DecodeError<()>) -> Error {
		Error::new(INVALID_CHARACTER_EXCEPTION, ErrorKind::Range)
	}

	let mut vec = Vec::with_capacity(decoded_len_estimate(data.len()));
	let mut decoder = Decoder::new(|bytes| {
		vec.extend_from_slice(bytes);
		Ok(())
	});
	match data {
		StringRef::Latin1(bytes) => {
			decoder.feed(bytes).map_err(decode_err)?;
			decoder.finish().map_err(decode_err)?;
		}
		StringRef::Utf16(wstr) => {
			for chunk in wstr.as_bytes().chunks_exact(2) {
				let codepoint = u16::from_ne_bytes([chunk[0], chunk[1]]);
				if codepoint > u8::MAX as u16 {
					return Err(Error::new(INVALID_CHARACTER_EXCEPTION, ErrorKind::Range));
				} else {
					decoder.feed(&[codepoint as u8]).map_err(decode_err)?;
				}
			}
			decoder.finish().map_err(decode_err)?;
		}
	}
	Ok(vec.into_iter().map(char::from).collect())
}

const FUNCTIONS: &[JSFunctionSpec] = &[function_spec!(btoa, 1), function_spec!(atob, 1), JSFunctionSpec::ZERO];

pub fn define(cx: &Context, global: &mut Object) -> bool {
	unsafe { global.define_methods(cx, FUNCTIONS) }
}
