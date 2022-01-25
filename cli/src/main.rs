/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use clap::Parser;

use runtime::config::{Config, CONFIG, LogLevel};

use crate::commands::{repl, run};
use crate::commands::eval;

mod commands;
pub mod evaluate;

#[derive(Parser)]
#[structopt(name = "spiderfire", about = "JavaScript Runtime")]
struct Cli {
	#[clap(subcommand)]
	command: Option<Command>,
}

#[derive(Parser)]
pub enum Command {
	#[clap(about = "Evaluates a line of JavaScript")]
	Eval {
		#[clap(help = "Line of JavaScript to be evaluated", required(true))]
		source: String,
	},

	#[clap(about = "Starts a JavaScript Shell")]
	Repl,

	#[clap(about = "Runs a JavaScript file")]
	Run {
		#[clap(help = "The JavaScript file to run, Default: 'main.js'", required(false), default_value = "main.js")]
		path: String,

		#[clap(help = "Sets logging level, Default: ERROR", short, long, required(false), default_value = "ERROR")]
		log_level: String,

		#[clap(help = "Sets logging level to DEBUG", short, long)]
		debug: bool,

		#[clap(help = "Disables ES Modules Features", short, long)]
		script: bool,
	},
}

fn main() {
	let args = Cli::parse();

	#[cfg(windows)]
	{
		colored::control::set_virtual_terminal(true).unwrap();
	}

	match args.command {
		Some(Command::Eval { source }) => {
			CONFIG.set(Config::default().log_level(LogLevel::Debug).script(true)).unwrap();
			eval::eval_source(&source);
		}

		Some(Command::Run { path, log_level, debug, script }) => {
			let log_level = if debug {
				LogLevel::Debug
			} else {
				match log_level.to_uppercase().as_str() {
					"NONE" => LogLevel::None,
					"INFO" => LogLevel::Info,
					"WARN" => LogLevel::Warn,
					"ERROR" => LogLevel::Error,
					"DEBUG" => LogLevel::Debug,
					_ => panic!("Invalid Logging Level"),
				}
			};

			CONFIG.set(Config::default().log_level(log_level).script(script)).unwrap();
			run::run(&path);
		}

		Some(Command::Repl) | None => {
			CONFIG.set(Config::default().log_level(LogLevel::Debug).script(true)).unwrap();
			repl::start_repl();
		}
	}
}
