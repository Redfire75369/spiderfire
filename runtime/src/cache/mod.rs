/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::OsStr;
use std::fs::{metadata, read_to_string};
use std::path::Path;

use sourcemap::SourceMap;

pub use cache::*;

mod cache;

pub fn locate_in_cache<P: AsRef<Path>>(path: P, script: &str) -> Option<(String, SourceMap)> {
	let result = Cache::new().map(|cache| {
		let path = path.as_ref();
		let folder = cache.find_folder(path)?;
		match cache.check_cache(path, &folder, script) {
			Ok(s) => Ok(s),
			Err(Error::HashedSource(hash)) => cache.save_to_cache(path, &folder, script, Some(&hash)),
			Err(Error::Other) => cache.save_to_cache(path, &folder, script, None),
			Err(err) => Err(err),
		}
	});

	match result {
		Some(Ok(s)) => Some(s),
		Some(Err(Error::HashedSource(_))) | Some(Err(Error::Other)) => None,
		Some(Err(err)) => {
			eprintln!("Error occurred while compiling TypeScript");
			eprintln!("{}", err);
			None
		}
		None => None,
	}
}

pub fn find_sourcemap<P: AsRef<Path>>(path: P) -> Option<SourceMap> {
	Cache::new().map(|cache| {
		let path = path.as_ref();
		let folder = cache.find_folder(path).ok()?;
		let source_file = path.file_stem().map(OsStr::to_str).flatten()?;
		let map_file = folder.join(source_file).with_extension("js.map");

		metadata(&map_file).ok()?.is_file().then(|| {
			let map = read_to_string(&map_file).ok()?;
			Some(SourceMap::from_slice(map.as_bytes()).unwrap())
		})?
	})?
}
