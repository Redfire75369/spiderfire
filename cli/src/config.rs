/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{io, result::Result};

use once_cell::sync::OnceCell;

pub static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Copy, Clone, Debug)]
pub struct Config {
	pub debug: bool,
	pub script: bool,
}

#[allow(clippy::unnecessary_wraps)]
impl Config {
	pub fn initialise(debug: bool, script: bool) -> Result<Config, io::Error> {
		let config = Config { debug, script };
		Ok(config)
	}

	pub fn global() -> &'static Config {
		CONFIG.get().expect("Configuration not initialised")
	}
}
