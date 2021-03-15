/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate mozjs;

use clap::{App, Arg, SubCommand};

use crate::cli::{repl, run};
use crate::config::{Config, CONFIG};

mod cli;
mod config;
pub(crate) mod runtime;

fn main() {
	let matches = App::new("Spiderfire")
		.version("0.1.0")
		.about("JavaScript Runtime")
		.subcommand(SubCommand::with_name("repl").about("Starts a JavaScript Shell"))
		.subcommand(
			SubCommand::with_name("run")
				.about("Runs a JavaScript File")
				.arg(
					Arg::with_name("PATH")
						.help("The JavaScript file to be run. Default: 'main.js'")
						.required(false),
				)
				.arg(Arg::with_name("debug").long("debug").help("Enables Debug Features").required(false))
				.arg(Arg::with_name("script").long("script").help("Disables ES Modules Features").required(false)),
		)
		.get_matches();

	if let Some(_) = matches.subcommand_matches("repl") {
		let config = Config::initialise(true, true).unwrap();
		CONFIG.set(config).unwrap();
		repl::start_repl();
	} else if let Some(subcmd) = matches.subcommand_matches("run") {
		let config = Config::initialise(subcmd.is_present("debug"), subcmd.is_present("script")).unwrap();
		CONFIG.set(config).unwrap();
		run::run(&String::from(subcmd.value_of("PATH").unwrap_or("./main.js")));
	}
}
