/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::{create_dir_all, metadata, read_dir};
use std::path::PathBuf;

use crate::cache::CACHE_DIR;

pub fn cache_statistics() {
	if let Some(cache_dir) = &*CACHE_DIR {
		create_dir_all(cache_dir).unwrap();
		println!("Location: {}", cache_dir.display());
		println!("Size: {}", format_size(cache_size(cache_dir)));
	} else {
		println!("No Cache Found");
	}
}

fn cache_size(folder: &PathBuf) -> u64 {
	let mut size: u64 = 0;
	let metadata = metadata(folder).unwrap();
	if metadata.is_dir() {
		for entry in read_dir(folder).unwrap() {
			if let Ok(entry) = entry {
				size += cache_size(&entry.path());
			}
		}
	} else {
		size += metadata.len();
	}
	size
}

const PREFIXES: [&str; 6] = ["", "Ki", "Mi", "Gi", "Ti", "Pi"];

fn format_size(size: u64) -> String {
	if size >= 1024 {
		let index: u32 = f64::log(size as f64, 1024.0).floor() as u32;
		let s1 = size / 1024_u64.pow(index);
		let s2 = (size - s1 * 1024_u64.pow(index)) / 1024_u64.pow(index - 1);

		if s2 != 0 {
			format!("{} {}B, {} {}B", s1, PREFIXES[index as usize], s2, PREFIXES[index as usize - 1])
		} else {
			format!("{} {}B", s1, PREFIXES[index as usize])
		}
	} else {
		format!("{} B", size)
	}
}
