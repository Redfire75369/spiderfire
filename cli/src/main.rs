/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate mozjs;

use clap::{App, Arg, SubCommand};

use runtime::config::{Config, CONFIG, LogLevel};

use crate::commands::{repl, run};
use crate::commands::eval;

mod commands;
pub mod evaluate;

fn main() {
	let matches = App::new("Spiderfire")
		.version("0.1.0")
		.about("JavaScript Runtime")
		.subcommand(
			SubCommand::with_name("eval")
				.about("Evaluates a line of JavaScript")
				.arg(Arg::with_name("source").help("Line of JavaScript to be evaluated").required(true)),
		)
		.subcommand(SubCommand::with_name("repl").about("Starts a JavaScript Shell"))
		.subcommand(
			SubCommand::with_name("run")
				.about("Runs a JavaScript File")
				.arg(
					Arg::with_name("path")
						.help("The JavaScript file to be run. Default: 'main.js'")
						.required(false),
				)
				.arg(
					Arg::with_name("log_level")
						.help("Sets Logging Level. Default: ERROR")
						.takes_value(true)
						.short("l")
						.long("loglevel")
						.required(false)
						.conflicts_with("debug"),
				)
				.arg(
					Arg::with_name("debug")
						.help("Sets Logging Level to DEBUG.")
						.short("d")
						.long("debug")
						.required(false),
				)
				.arg(
					Arg::with_name("script")
						.help("Disables ES Modules Features")
						.long("script")
						.required(false),
				),
		)
		.get_matches();

	match matches.subcommand_name() {
		Some("eval") => {
			let subcmd = matches.subcommand_matches("run").unwrap();

			let config = Config::initialise(LogLevel::Debug, true).unwrap();
			CONFIG.set(config).unwrap();
			eval::eval_source(subcmd.value_of("source").unwrap());
		}
		Some("repl") => {
			let config = Config::initialise(LogLevel::Debug, true).unwrap();
			CONFIG.set(config).unwrap();
			repl::start_repl();
		}
		Some("run") => {
			let subcmd = matches.subcommand_matches("run").unwrap();

			let log_level = if subcmd.is_present("debug") {
				LogLevel::Debug
			} else if let Some(level) = subcmd.value_of("log_level") {
				let level = level.to_uppercase();
				match level.as_str() {
					"NONE" => LogLevel::None,
					"INFO" => LogLevel::Info,
					"WARN" => LogLevel::Warn,
					"ERROR" => LogLevel::Error,
					"DEBUG" => LogLevel::Debug,
					_ => panic!("Invalid Logging Level"),
				}
			} else {
				LogLevel::Error
			};

			let config = Config::initialise(log_level, subcmd.is_present("script")).unwrap();
			CONFIG.set(config).unwrap();
			run::run(&String::from(subcmd.value_of("path").unwrap_or("./main.js")));
		}
		_ => (),
	}
}
