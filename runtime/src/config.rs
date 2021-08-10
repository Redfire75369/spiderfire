/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use once_cell::sync::OnceCell;

pub static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
			LogLevel::None | LogLevel::Info | LogLevel::Debug => true,
			LogLevel::Warn | LogLevel::Error => false,
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

impl Config {
	pub fn log_level(self, log_level: LogLevel) -> Config {
		Config { log_level, ..self }
	}

	pub fn script(self, script: bool) -> Config {
		Config { script, ..self }
	}

	pub fn global() -> &'static Config {
		CONFIG.get().expect("Configuration not initialised")
	}
}

impl Default for Config {
	fn default() -> Config {
		Config {
			log_level: LogLevel::Error,
			script: false,
		}
	}
}
