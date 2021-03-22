/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{io, result::Result};

use once_cell::sync::OnceCell;

#[derive(Copy, Clone, Debug)]
pub(crate) struct Config {
	pub(crate) debug: bool,
	pub(crate) script: bool,
}

impl Config {
	pub(crate) fn initialise(debug: bool, script: bool) -> Result<Config, io::Error> {
		let config = Config { debug, script };
		return Ok(config);
	}

	pub(crate) fn global() -> &'static Config {
		CONFIG.get().expect("Configuration not initialised")
	}
}

pub(crate) static CONFIG: OnceCell<Config> = OnceCell::new();
