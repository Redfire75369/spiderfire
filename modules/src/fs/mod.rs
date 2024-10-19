/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */
use std::io;

pub use fs::*;
pub use handle::*;
use ion::Error;

mod fs;
mod handle;

pub(crate) fn base_error(base: &str, path: &str, err: io::Error) -> Error {
	Error::new(format!("Could not {} {}: {}", base, path, err), None)
}

pub(crate) fn file_error(action: &str, path: &str, err: io::Error) -> Error {
	Error::new(format!("Could not {} file {}: {}", action, path, err), None)
}

pub(crate) fn dir_error(action: &str, path: &str, err: io::Error) -> Error {
	Error::new(format!("Could not {} directory {}: {}", action, path, err), None)
}

pub(crate) fn translate_error(action: &str, from: &str, to: &str, err: io::Error) -> Error {
	Error::new(format!("Could not {} {} to {}: {}", action, from, to, err), None)
}
