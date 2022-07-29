/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::{create_dir_all, read_dir, read_to_string, remove_dir_all, write};
use std::path::{Path, PathBuf};

use base64_url::encode;
use dirs::home_dir;
use os_str_bytes::OsStrBytes;
use sha3::{Digest, Sha3_512};

use runtime::config::Config;
use runtime::typescript::compile_typescript;

lazy_static! {
	pub static ref CACHE_DIR: Option<PathBuf> = home_dir().map(|path| path.join(".spiderfire").join("cache"));
}

pub enum CacheMiss {
	Partial(PathBuf, Option<String>),
	None,
	NoCache,
}

pub fn check_cache(path: &Path, source: &str) -> Result<String, CacheMiss> {
	let canonical = path.canonicalize().unwrap();
	let folder_path = canonical.parent().unwrap();
	let folder_name = path.parent().unwrap().file_name().unwrap().to_str().unwrap();
	let source_file = path.file_stem().unwrap().to_str().unwrap();
	let extension = path.extension().unwrap().to_str().unwrap();

	let folder_hash = hash(folder_path.to_raw_bytes());
	let file_hash = hash(source);

	let folder_path = format!("{}-{}", folder_name, &folder_hash[0..=16]);
	let file_path = format!("{}.js", source_file);
	let hash_path = format!("{}.{}.sha512", source_file, extension);

	if let Some(cache_dir) = &*CACHE_DIR {
		let cache_path = cache_dir.join(&folder_path);
		if cache_path.is_dir() {
			if cache_path.join(&hash_path).exists() {
				if cache_path.join(&file_path).exists() {
					let hash = read_to_string(&cache_path.join(&hash_path)).unwrap();
					return if hash.trim() == file_hash.trim() {
						Ok(read_to_string(&cache_path.join(&file_path)).unwrap())
					} else {
						Err(CacheMiss::Partial(cache_path, Some(file_hash)))
					};
				}
			}
			Err(CacheMiss::Partial(cache_path, None))
		} else {
			Err(CacheMiss::None)
		}
	} else {
		Err(CacheMiss::NoCache)
	}
}

pub fn save_in_cache(path: &Path, source: &str, cache_path: Option<PathBuf>, file_hash: Option<String>) -> Option<String> {
	if let Some(cache_dir) = &*CACHE_DIR {
		if Config::global().typescript {
			let cache_path = cache_path.unwrap_or_else(|| {
				let canonical = path.canonicalize().unwrap();
				let folder_path = canonical.parent().unwrap();
				let folder_name = path.parent().unwrap().file_name().unwrap().to_str().unwrap();
				let folder_hash = hash(folder_path.to_raw_bytes());

				let folder_path = format!("{}-{}", folder_name, &folder_hash[0..=16]);
				cache_dir.join(&folder_path)
			});
			create_dir_all(&cache_path).unwrap();

			let source_file = path.file_stem().unwrap().to_str().unwrap();
			let extension = path.extension().unwrap().to_str().unwrap();
			let file_hash = file_hash.unwrap_or_else(|| hash(source));

			let file_path = format!("{}.js", source_file);
			let hash_path = format!("{}.{}.sha512", source_file, extension);

			let dst = compile_typescript(path.file_name().unwrap().to_str().unwrap(), &source);
			write(cache_path.join(&file_path), &dst).unwrap();
			write(cache_path.join(&hash_path), &file_hash).unwrap();
			Some(dst)
		} else {
			None
		}
	} else {
		None
	}
}

pub fn clear_cache() {
	if let Some(cache_dir) = &*CACHE_DIR {
		for entry in read_dir(&cache_dir).unwrap() {
			if let Ok(entry) = entry {
				remove_dir_all(entry.path()).unwrap();
			}
		}
		create_dir_all(&cache_dir).unwrap();
	}
}

fn hash<T: AsRef<[u8]>>(bytes: T) -> String {
	encode(&Sha3_512::new().chain_update(bytes).finalize())
}
