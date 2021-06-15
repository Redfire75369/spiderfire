/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{io::Error, result::Result};

use once_cell::sync::OnceCell;

pub static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LogLevel {
	None = 0,
	Info = 1,
	Warn = 2,
	Error = 3,
	Debug = 4,
}

impl LogLevel {
	pub fn is_stdout(&self) -> bool {
		match self {
			LogLevel::None | LogLevel::Info | LogLevel::Debug => false,
			LogLevel::Warn | LogLevel::Error => true,
		}
	}

	pub fn is_stderr(&self) -> bool {
		!self.is_stdout()
	}
}

#[derive(Copy, Clone, Debug)]
pub struct Config {
	pub log_level: LogLevel,
	pub script: bool,
}

#[allow(clippy::unnecessary_wraps)]
impl Config {
	pub fn initialise(log_level: LogLevel, script: bool) -> Result<Config, Error> {
		let config = Config { log_level, script };

		Ok(config)
	}

	pub fn global() -> &'static Config {
		CONFIG.get().expect("Configuration not initialised")
	}
}
