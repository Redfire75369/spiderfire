/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use encoding_rs::{Encoder, UTF_8};

use ion::{Context, Object, Value};
use ion::class::Reflector;
use ion::conversions::ToValue;
use ion::typedarray::Uint8Array;

pub struct EncodeResult {
	read: u64,
	written: u64,
}

impl<'cx> ToValue<'cx> for EncodeResult {
	fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		let mut object = Object::new(cx);
		object.set_as(cx, "read", &self.read);
		object.set_as(cx, "written", &self.written);
		object.to_value(cx, value);
	}
}

#[js_class]
pub struct TextEncoder {
	reflector: Reflector,
	#[ion(no_trace)]
	encoder: Encoder,
}

#[js_class]
impl TextEncoder {
	#[ion(constructor)]
	pub fn constructor() -> TextEncoder {
		TextEncoder {
			reflector: Reflector::default(),
			encoder: UTF_8.new_encoder(),
		}
	}

	pub fn encode(&mut self, input: Option<String>) -> Uint8Array {
		let input = input.unwrap_or_default();
		let mut buf = Vec::with_capacity(self.encoder.max_buffer_length_from_utf8_if_no_unmappables(input.len()).unwrap());
		let (_, _, _) = self.encoder.encode_from_utf8_to_vec(&input, &mut buf, true);
		Uint8Array::from(buf)
	}

	#[ion(name = "encodeInto")]
	pub fn encode_into(&mut self, input: String, destination: mozjs::typedarray::Uint8Array) -> EncodeResult {
		let mut destination = destination;
		let (_, read, written, _) = self.encoder.encode_from_utf8(&input, unsafe { destination.as_mut_slice() }, true);
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
