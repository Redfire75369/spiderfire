/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub use decode::*;
pub use encode::*;
use ion::{ClassInitialiser, Context, Object, Value};
use ion::conversions::ToValue;

#[derive(Default, FromValue)]
pub struct TextDecoderOptions {
	#[ion(default)]
	fatal: bool,
	#[ion(default, name = "ignoreBOM")]
	ignore_byte_order_mark: bool,
}

#[derive(Default, FromValue)]
pub struct TextDecodeOptions {
	#[ion(default)]
	stream: bool,
}

pub struct EncodeResult {
	read: u64,
	written: u64,
}

impl<'cx> ToValue<'cx> for EncodeResult {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		let mut object = Object::new(cx);
		object.set_as(cx, "read", &self.read);
		object.set_as(cx, "written", &self.written);
		object.to_value(cx, value);
	}
}

#[js_class]
mod decode {
	use encoding_rs::{Decoder, DecoderResult, Encoding, UTF_8};
	use mozjs::typedarray::ArrayBufferView;

	use ion::{Error, ErrorKind, Result};

	use crate::globals::encoding::{TextDecodeOptions, TextDecoderOptions};

	pub struct TextDecoder {
		decoder: Decoder,
		#[ion(readonly)]
		pub fatal: bool,
		#[ion(readonly, name = "ignoreBOM")]
		pub ignore_byte_order_mark: bool,
	}

	impl TextDecoder {
		#[ion(constructor)]
		pub fn constructor(label: Option<String>, options: Option<TextDecoderOptions>) -> Result<TextDecoder> {
			let encoding;
			if let Some(label) = label {
				let enc = Encoding::for_label_no_replacement(label.as_bytes());
				match enc {
					None => return Err(Error::new(&format!("The given encoding '{}' is not supported.", label), ErrorKind::Range)),
					Some(enc) => encoding = enc,
				}
			} else {
				encoding = UTF_8;
			}

			let options = options.unwrap_or_default();
			let decoder = if options.ignore_byte_order_mark {
				encoding.new_decoder_without_bom_handling()
			} else {
				encoding.new_decoder()
			};

			Ok(TextDecoder {
				decoder,
				fatal: options.fatal,
				ignore_byte_order_mark: options.ignore_byte_order_mark,
			})
		}

		pub unsafe fn decode(&mut self, buffer: ArrayBufferView, options: Option<TextDecodeOptions>) -> Result<String> {
			let mut string = String::with_capacity(self.decoder.max_utf8_buffer_length(buffer.len()).unwrap());
			let stream = options.unwrap_or_default().stream;
			if self.fatal {
				let (result, _) = self.decoder.decode_to_string_without_replacement(buffer.as_slice(), &mut string, !stream);
				if let DecoderResult::Malformed(_, _) = result {
					return Err(Error::new("TextDecoder.decode: Decoding Failed", ErrorKind::Type));
				}
			} else {
				let (_, _, _) = self.decoder.decode_to_string(buffer.as_slice(), &mut string, !stream);
			}
			Ok(string)
		}

		#[ion(get)]
		pub fn get_encoding(&self) -> String {
			String::from(self.decoder.encoding().name())
		}
	}
}

#[js_class]
mod encode {
	use encoding_rs::{Encoder, UTF_8};

	use ion::typedarray::Uint8Array;

	use crate::globals::encoding::EncodeResult;

	pub struct TextEncoder {
		encoder: Encoder,
	}

	impl TextEncoder {
		#[ion(constructor)]
		pub fn constructor() -> TextEncoder {
			TextEncoder { encoder: UTF_8.new_encoder() }
		}

		pub fn encode(&mut self, input: Option<String>) -> Uint8Array {
			let input = input.unwrap_or_default();
			let mut buf = Vec::with_capacity(self.encoder.max_buffer_length_from_utf8_if_no_unmappables(input.len()).unwrap());
			let (_, _, _) = self.encoder.encode_from_utf8_to_vec(&input, &mut buf, true);
			Uint8Array { buf }
		}

		pub unsafe fn encodeInto(&mut self, input: String, destination: mozjs::typedarray::Uint8Array) -> EncodeResult {
			let mut destination = destination;
			let (_, read, written, _) = self.encoder.encode_from_utf8(&input, destination.as_mut_slice(), true);
			EncodeResult {
				read: read as u64,
				written: written as u64,
			}
		}

		#[ion(get)]
		pub fn get_encoding(&self) -> String {
			String::from(self.encoder.encoding().name())
		}
	}
}

pub fn define(cx: &Context, global: &mut Object) -> bool {
	TextDecoder::init_class(cx, global);
	TextEncoder::init_class(cx, global);
	true
}
