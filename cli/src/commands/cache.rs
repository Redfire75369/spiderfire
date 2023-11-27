/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::{metadata, read_dir};
use std::io;
use std::path::Path;

use humansize::{BINARY, SizeFormatter};

use runtime::cache::Cache;

pub(crate) fn cache_statistics() {
	if let Some(cache) = Cache::new() {
		println!("Location: {}", cache.dir().display());
		match cache_size(cache.dir()) {
			Ok(size) => println!("Size: {}", SizeFormatter::new(size, BINARY)),
			Err(err) => eprintln!("Error while Calculating Size: {}", err),
		}
	} else {
		println!("No Cache Found");
	}
}

fn cache_size(folder: &Path) -> io::Result<u64> {
	let mut size = 0;
	let metadata = metadata(folder)?;
	if metadata.is_dir() {
		for entry in read_dir(folder)? {
			size += cache_size(&entry?.path())?;
		}
	} else {
		size += metadata.len();
	}
	Ok(size)
}
