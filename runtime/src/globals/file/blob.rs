/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::str::FromStr;

use bytes::Bytes;
use encoding_rs::UTF_8;
use mozjs::conversions::ConversionBehavior;
use mozjs::jsapi::JSObject;

use ion::{ClassDefinition, Context, Error, ErrorKind, Object, Promise, Result, Value};
use ion::class::Reflector;
use ion::conversions::FromValue;
use ion::format::NEWLINE;
use ion::typedarray::{ArrayBuffer, ArrayBufferView, ArrayBufferWrapper};

use crate::promise::future_to_promise;

pub enum BufferSource<'cx> {
	Buffer(ArrayBuffer<'cx>),
	View(ArrayBufferView<'cx>),
}

impl BufferSource<'_> {
	pub fn len(&self) -> usize {
		unsafe { self.as_slice().len() }
	}

	pub fn is_shared(&self, cx: &Context) -> bool {
		match self {
			BufferSource::Buffer(buffer) => buffer.is_shared(),
			BufferSource::View(view) => view.is_shared(cx),
		}
	}

	pub unsafe fn as_slice(&self) -> &[u8] {
		match self {
			BufferSource::Buffer(buffer) => unsafe { buffer.as_slice() },
			BufferSource::View(view) => unsafe { view.as_slice() },
		}
	}

	pub fn to_vec(&self) -> Vec<u8> {
		unsafe { self.as_slice().to_vec() }
	}

	pub fn to_bytes(&self) -> Bytes {
		Bytes::copy_from_slice(unsafe { self.as_slice() })
	}
}

impl<'cx> FromValue<'cx> for BufferSource<'cx> {
	type Config = bool;
	fn from_value(cx: &'cx Context, value: &Value, strict: bool, allow_shared: bool) -> Result<BufferSource<'cx>> {
		let obj = Object::from_value(cx, value, strict, ())?;
		if let Some(buffer) = ArrayBuffer::from(cx.root_object(obj.handle().get())) {
			if buffer.is_shared() && !allow_shared {
				return Err(Error::new("Buffer Source cannot be shared", ErrorKind::Type));
			}
			Ok(BufferSource::Buffer(buffer))
		} else if let Some(view) = ArrayBufferView::from(obj.into_local()) {
			if view.is_shared(cx) && !allow_shared {
				return Err(Error::new("Buffer Source cannot be shared", ErrorKind::Type));
			}
			Ok(BufferSource::View(view))
		} else {
			Err(Error::new("Object is not a buffer source.", ErrorKind::Type))
		}
	}
}

#[derive(Clone, Debug, Default)]
pub struct BlobPart(Bytes);

impl<'cx> FromValue<'cx> for BlobPart {
	type Config = ();
	fn from_value(cx: &'cx Context, value: &Value, strict: bool, _: ()) -> Result<BlobPart> {
		if value.handle().is_string() {
			return Ok(BlobPart(Bytes::from(String::from_value(cx, value, true, ())?)));
		} else if value.handle().is_object() {
			if let Ok(buffer_source) = BufferSource::from_value(cx, value, strict, false) {
				return Ok(BlobPart(buffer_source.to_bytes()));
			} else if let Ok(blob) = <&Blob>::from_value(cx, value, strict, ()) {
				return Ok(BlobPart(blob.bytes.clone()));
			}
		}
		Err(Error::new(
			"Expected BufferSource, Blob or String in Blob constructor.",
			ErrorKind::Type,
		))
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Endings {
	#[default]
	Transparent,
	Native,
}

impl FromStr for Endings {
	type Err = Error;

	fn from_str(endings: &str) -> Result<Endings> {
		match endings {
			"transparent" => Ok(Endings::Transparent),
			"native" => Ok(Endings::Native),
			_ => Err(Error::new("Invalid ending type for Blob", ErrorKind::Type)),
		}
	}
}

impl<'cx> FromValue<'cx> for Endings {
	type Config = ();

	fn from_value(cx: &'cx Context, value: &Value, strict: bool, _: ()) -> Result<Endings> {
		let endings = String::from_value(cx, value, strict, ())?;
		Endings::from_str(&endings)
	}
}

#[derive(Debug, Default, FromValue)]
pub struct BlobOptions {
	#[ion(name = "type")]
	kind: Option<String>,
	#[ion(default)]
	endings: Endings,
}

#[js_class]
pub struct Blob {
	reflector: Reflector,
	#[trace(no_trace)]
	bytes: Bytes,
	kind: Option<String>,
}

impl Blob {
	pub fn as_bytes(&self) -> &Bytes {
		&self.bytes
	}

	pub fn kind(&self) -> Option<String> {
		self.kind.clone()
	}
}

#[js_class]
impl Blob {
	#[ion(constructor)]
	pub fn constructor(parts: Option<Vec<BlobPart>>, options: Option<BlobOptions>) -> Blob {
		let options = options.unwrap_or_default();

		let mut bytes = Vec::new();

		if let Some(parts) = parts {
			let len = parts
				.iter()
				.map(|part| part.0.len() + part.0.iter().filter(|&&b| b == b'\r' || b == b'\n').count() * 2)
				.sum();
			bytes.reserve(len);

			for part in parts {
				match options.endings {
					Endings::Transparent => bytes.extend(part.0),
					Endings::Native => {
						let mut i = 0;
						while let Some(&b) = part.0.get(i) {
							i += 1;
							if b == b'\r' {
								bytes.extend_from_slice(NEWLINE.as_bytes());
								if part.0.get(i) == Some(&b'\n') {
									i += 1;
								}
							} else if b == b'\n' {
								bytes.extend_from_slice(NEWLINE.as_bytes());
							} else {
								bytes.push(b);
							}
						}
					}
				}
			}
		}

		Blob {
			reflector: Reflector::default(),
			bytes: Bytes::from(bytes),
			kind: options.kind,
		}
	}

	#[ion(get)]
	pub fn get_size(&self) -> u64 {
		self.bytes.len() as u64
	}

	#[ion(get)]
	pub fn get_type(&self) -> String {
		self.kind.clone().unwrap_or_default()
	}

	pub fn slice(
		&self, cx: &Context, #[ion(convert = ConversionBehavior::Clamp)] start: Option<i64>,
		#[ion(convert = ConversionBehavior::Clamp)] end: Option<i64>, kind: Option<String>,
	) -> *mut JSObject {
		let size = self.bytes.len() as i64;

		let mut start = start.unwrap_or(0);
		if start < 0 {
			start = 0.max(size + start);
		}
		let start = start.min(size) as usize;

		let mut end = end.unwrap_or(size);
		if end < 0 {
			end = 0.max(size + end);
		}
		let end = end.min(size) as usize;

		let kind = match kind {
			Some(mut kind) if kind.as_bytes().iter().all(|&b| (0x20..=0x7E).contains(&b)) => {
				kind.make_ascii_lowercase();
				Some(kind)
			}
			_ => None,
		};

		let span = 0.max(end - start);

		let bytes = self.bytes.slice(start..start + span);

		let blob = Blob {
			reflector: Reflector::default(),
			bytes,
			kind,
		};
		Blob::new_object(cx, Box::new(blob))
	}

	pub fn text<'cx>(&self, cx: &'cx Context) -> Option<Promise<'cx>> {
		let bytes = self.bytes.clone();
		future_to_promise(cx, async move { Ok::<_, ()>(UTF_8.decode(&bytes).0.into_owned()) })
	}

	#[ion(name = "arrayBuffer")]
	pub fn array_buffer<'cx>(&self, cx: &'cx Context) -> Option<Promise<'cx>> {
		let bytes = self.bytes.clone();
		future_to_promise(cx, async move { Ok::<_, ()>(ArrayBufferWrapper::from(bytes.to_vec())) })
	}
}
