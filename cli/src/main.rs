/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use structopt::StructOpt;
use runtime::config::{Config, CONFIG, LogLevel};
use crate::commands::{repl, run};
use crate::commands::eval;

mod commands;
pub mod evaluate;

#[derive(StructOpt)]
#[structopt(name = "spiderfire", about = "JavaScript Runtime")]
struct Cli {
	#[structopt(subcommand)]
	commands: Option<Commands>,
}

#[derive(StructOpt)]
pub enum Commands {
	#[structopt(about = "Evaluates a line of JavaScript")]
	Eval {
		#[structopt(required(true), about = "Line of JavaScript to be evaluated")]
		source: String,
	},

	#[structopt(about = "Starts a JavaScript Shell")]
	Repl,

	#[structopt(about = "Runs a JavaScript file")]
	Run {
		#[structopt(about = "The JavaScript file to run. Default: 'main.js'", required(false), default_value = "main.js")]
		path: String,

		#[structopt(about = "Sets logging level, Default: ERROR", short, long, required(false), default_value = "error")]
		log_level: String,

		#[structopt(about = "Sets logging level to DEBUG.", short, long)]
		debug: bool,

		#[structopt(about = "Disables ES Modules Features", short, long)]
		script: bool,
	},
}

fn main() {
	let args = Cli::from_args();

	match args.commands {
		Some(Eval { source }) => {
			CONFIG
				.set(Config::default().log_level(LogLevel::Debug).script(true))
				.expect("Config Initialisation Failed");
			eval::eval_source(source);
		}

		Some(Run {
			path,
			log_level,
			debug,
			script,
		}) => {

			let log_lev = if debug {
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

			CONFIG
				.set(Config::default().log_level(log_lev).script(script))
				.expect("Config Initialisation Failed");
			run::run(path);
		}

		Some(Repl) | None => {
			CONFIG
				.set(Config::default().log_level(LogLevel::Debug).script(true))
				.expect("Config Initialisation Failed");
			repl::start_repl();
		}
	}
}
