/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use chrono::{DateTime, TimeZone, Utc};
use mozjs::conversions::ConversionBehavior;

pub use blob::{Blob, buffer_source_to_bytes};
use ion::{ClassDefinition, Context, Object};

use crate::globals::file::blob::{BlobOptions, BlobPart};
use crate::globals::file::reader::{FileReader, FileReaderSync};

mod blob;
mod reader;

#[derive(Debug, Default, FromValue)]
pub struct FileOptions {
	#[ion(inherit)]
	blob: BlobOptions,
	#[ion(convert = ConversionBehavior::Default)]
	modified: Option<i64>,
}

#[js_class]
pub struct File {
	blob: Blob,
	name: String,
	#[ion(no_trace)]
	modified: DateTime<Utc>,
}

#[js_class]
impl File {
	#[ion(constructor)]
	pub fn constructor(parts: Vec<BlobPart>, name: String, options: Option<FileOptions>) -> File {
		let options = options.unwrap_or_default();
		let blob = Blob::constructor(Some(parts), Some(options.blob));
		let modified = options
			.modified
			.and_then(|d| Utc.timestamp_millis_opt(d).single())
			.unwrap_or_else(Utc::now);

		File { blob, name, modified }
	}

	#[ion(get)]
	pub fn get_name(&self) -> &str {
		&self.name
	}

	#[ion(get)]
	pub fn get_last_modified(&self) -> i64 {
		self.modified.timestamp_millis()
	}
}

pub fn define(cx: &Context, object: &mut Object) -> bool {
	Blob::init_class(cx, object).0
		&& File::init_class(cx, object).0
		&& FileReader::init_class(cx, object).0
		&& FileReaderSync::init_class(cx, object).0
}
