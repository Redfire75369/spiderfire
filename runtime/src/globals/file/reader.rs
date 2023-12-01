/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ptr;
use std::str::FromStr;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use encoding_rs::{Encoding, UTF_8};
use mime::Mime;
use mozjs::jsapi::JSObject;
use mozjs::jsval::{JSVal, NullValue};

use ion::{ClassDefinition, Context, Error, ErrorKind, Object, Result, Value};
use ion::class::{NativeObject, Reflector};
use ion::conversions::ToValue;
use ion::string::byte::{ByteString, Latin1};
use ion::typedarray::ArrayBuffer;

use crate::globals::file::Blob;
use crate::promise::future_to_promise;

fn encoding_from_string_mime(encoding: Option<&str>, mime: Option<&str>) -> &'static Encoding {
	encoding
		.and_then(|e| match Encoding::for_label_no_replacement(e.as_bytes()) {
			None if mime.is_some() => Mime::from_str(mime.unwrap()).ok().and_then(|mime| {
				Encoding::for_label_no_replacement(
					mime.get_param("charset").map(|p| p.as_str().as_bytes()).unwrap_or(b""),
				)
			}),
			e => e,
		})
		.unwrap_or(UTF_8)
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Traceable)]
#[repr(u8)]
pub enum FileReaderState {
	#[default]
	Empty = 0,
	Loading = 1,
	Done = 2,
}

impl FileReaderState {
	fn validate(&mut self) -> Result<()> {
		if FileReaderState::Loading == *self {
			return Err(Error::new("Invalid State for File Reader", ErrorKind::Type));
		}
		*self = FileReaderState::Loading;
		Ok(())
	}
}

#[derive(Debug, Default)]
#[js_class]
pub struct FileReader {
	reflector: Reflector,
	state: FileReaderState,
	#[ion(no_trace)]
	result: Option<Value>,
	#[ion(no_trace)]
	error: Option<Object>,
}

#[js_class]
impl FileReader {
	pub const EMPTY: i32 = FileReaderState::Empty as u8 as i32;
	pub const LOADING: i32 = FileReaderState::Loading as u8 as i32;
	pub const DONE: i32 = FileReaderState::Done as u8 as i32;

	#[ion(constructor)]
	pub fn constructor() -> FileReader {
		FileReader::default()
	}

	#[ion(get)]
	pub fn get_ready_state(&self) -> u8 {
		self.state as u8
	}

	#[ion(get)]
	pub fn get_result(&self) -> JSVal {
		self.result.as_ref().map(|v| v.get()).unwrap_or_else(NullValue)
	}

	#[ion(get)]
	pub fn get_error(&self) -> *mut JSObject {
		self.error.as_ref().map(|o| o.handle().get()).unwrap_or_else(ptr::null_mut)
	}

	fn read_result<T: ToValue>(&mut self, cx: &Context, result: T) {
		match result.to_value(cx) {
			Ok(value) => self.result = Some(value),
			Err(error) => self.error = Some(error.to_object(cx).unwrap()),
		}
	}

	#[ion(name = "readAsArrayBuffer")]
	pub fn read_as_array_buffer(&mut self, cx: &Context, blob: *const Blob) -> Result<()> {
		self.state.validate()?;
		let blob = unsafe { &*blob };
		let bytes = blob.as_bytes().clone();

		let cx2 = unsafe { Context::new_unchecked(cx.as_ptr()) };
		let mut this = Object::from(cx.root(self.reflector().get()));
		future_to_promise(cx, async move {
			let reader = FileReader::get_mut_private(&mut this);
			let array_buffer = ArrayBuffer::from(bytes.to_vec());
			reader.read_result(&cx2, array_buffer);
			Ok::<_, ()>(())
		});
		Ok(())
	}

	#[ion(name = "readAsBinaryString")]
	pub fn read_as_binary_string(&mut self, cx: &Context, blob: *const Blob) -> Result<()> {
		self.state.validate()?;
		let blob = unsafe { &*blob };
		let bytes = blob.as_bytes().clone();

		let cx2 = unsafe { Context::new_unchecked(cx.as_ptr()) };
		let mut this = Object::from(cx.root(self.reflector().get()));
		future_to_promise(cx, async move {
			let reader = FileReader::get_mut_private(&mut this);
			let byte_string = unsafe { ByteString::<Latin1>::from_unchecked(bytes.to_vec()) };
			reader.read_result(&cx2, byte_string);
			Ok::<_, ()>(())
		});
		Ok(())
	}

	#[ion(name = "readAsText")]
	pub fn read_as_text(&mut self, cx: &Context, blob: *const Blob, encoding: Option<String>) -> Result<()> {
		self.state.validate()?;
		let blob = unsafe { &*blob };
		let bytes = blob.as_bytes().clone();
		let mime = blob.kind();

		let cx2 = unsafe { Context::new_unchecked(cx.as_ptr()) };
		let mut this = Object::from(cx.root(self.reflector().get()));
		future_to_promise(cx, async move {
			let encoding = encoding_from_string_mime(encoding.as_deref(), mime.as_deref());

			let reader = FileReader::get_mut_private(&mut this);
			let str = encoding.decode_without_bom_handling(&bytes).0;
			reader.read_result(&cx2, str);
			Ok::<_, ()>(())
		});
		Ok(())
	}

	#[ion(name = "readAsDataURL")]
	pub fn read_as_data_url(&mut self, cx: &Context, blob: *const Blob) -> Result<()> {
		self.state.validate()?;
		let blob = unsafe { &*blob };
		let bytes = blob.as_bytes().clone();
		let mime = blob.kind();

		let cx2 = unsafe { Context::new_unchecked(cx.as_ptr()) };
		let mut this = Object::from(cx.root(self.reflector().get()));
		future_to_promise(cx, async move {
			let reader = FileReader::get_mut_private(&mut this);
			let base64 = BASE64_STANDARD.encode(&bytes);
			let data_url = match mime {
				Some(mime) => format!("data:{};base64,{}", mime, base64),
				None => format!("data:base64,{}", base64),
			};
			reader.read_result(&cx2, data_url);
			Ok::<_, ()>(())
		});
		Ok(())
	}
}

#[derive(Debug, Default)]
#[js_class]
pub struct FileReaderSync {
	reflector: Reflector,
}

#[js_class]
impl FileReaderSync {
	#[ion(constructor)]
	pub fn constructor() -> FileReaderSync {
		FileReaderSync::default()
	}

	#[ion(name = "readAsArrayBuffer")]
	pub fn read_as_array_buffer(&mut self, blob: *const Blob) -> ArrayBuffer {
		let blob = unsafe { &*blob };
		ArrayBuffer::from(blob.as_bytes().to_vec())
	}

	#[ion(name = "readAsBinaryString")]
	pub fn read_as_binary_string(&mut self, blob: *const Blob) -> ByteString {
		let blob = unsafe { &*blob };
		unsafe { ByteString::<Latin1>::from_unchecked(blob.as_bytes().to_vec()) }
	}

	#[ion(name = "readAsText")]
	pub fn read_as_text(&mut self, blob: *const Blob, encoding: Option<String>) -> String {
		let blob = unsafe { &*blob };
		let encoding = encoding_from_string_mime(encoding.as_deref(), blob.kind().as_deref());
		encoding.decode_without_bom_handling(blob.as_bytes()).0.into_owned()
	}

	#[ion(name = "readAsDataURL")]
	pub fn read_as_data_url(&mut self, blob: *const Blob) -> String {
		let blob = unsafe { &*blob };
		let mime = blob.kind();

		let base64 = BASE64_STANDARD.encode(blob.as_bytes());
		match mime {
			Some(mime) => format!("data:{};base64,{}", mime, base64),
			None => format!("data:base64,{}", base64),
		}
	}
}
