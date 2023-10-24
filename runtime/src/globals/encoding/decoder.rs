/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use encoding_rs::{Decoder, DecoderResult, Encoding, UTF_8};
use mozjs::typedarray::ArrayBufferView;

use ion::{Error, ErrorKind, Result};
use ion::class::Reflector;

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

#[js_class]
pub struct TextDecoder {
	reflector: Reflector,
	#[ion(no_trace)]
	decoder: Decoder,
	#[ion(readonly)]
	pub fatal: bool,
	#[ion(readonly, name = "ignoreBOM")]
	pub ignore_byte_order_mark: bool,
}

#[js_class]
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
			reflector: Reflector::default(),
			decoder,
			fatal: options.fatal,
			ignore_byte_order_mark: options.ignore_byte_order_mark,
		})
	}

	pub fn decode(&mut self, buffer: ArrayBufferView, options: Option<TextDecodeOptions>) -> Result<String> {
		let mut string = String::with_capacity(self.decoder.max_utf8_buffer_length(buffer.len()).unwrap());
		let stream = options.unwrap_or_default().stream;
		if self.fatal {
			let (result, _) = self
				.decoder
				.decode_to_string_without_replacement(unsafe { buffer.as_slice() }, &mut string, !stream);
			if let DecoderResult::Malformed(_, _) = result {
				return Err(Error::new("TextDecoder.decode: Decoding Failed", ErrorKind::Type));
			}
		} else {
			let (_, _, _) = self.decoder.decode_to_string(unsafe { buffer.as_slice() }, &mut string, !stream);
		}
		Ok(string)
	}

	#[ion(get)]
	pub fn get_encoding(&self) -> String {
		String::from(self.decoder.encoding().name())
	}
}
