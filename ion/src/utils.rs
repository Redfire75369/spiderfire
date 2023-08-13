/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::{Component, Path, PathBuf};

pub use send_wrapper::SendWrapper;

/// Normalises a [Path] by removing all `./` and resolving all `../` simplistically.
/// This function does not follow symlinks and may result in unexpected behaviour.
pub fn normalise_path<P: AsRef<Path>>(path: P) -> PathBuf {
	let mut buf = PathBuf::new();
	let segments = path.as_ref().components();

	for segment in segments {
		match segment {
			Component::ParentDir => {
				let len = buf.components().count();
				if len == 0 || buf.components().all(|c| matches!(c, Component::ParentDir)) {
					buf.push("..");
				} else {
					buf.pop();
				}
			}
			Component::CurDir => {}
			segment => buf.push(segment),
		}
	}
	buf
}
