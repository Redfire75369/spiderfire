/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate rustyline_derive;

use clap::Parser;

use crate::commands::handle_command;

mod cache;
mod commands;
mod evaluate;
mod repl;

#[derive(Parser)]
#[structopt(name = "spiderfire", about = "JavaScript Runtime")]
struct Cli {
	#[clap(subcommand)]
	command: Option<Command>,
}

#[derive(Parser)]
pub enum Command {
	#[clap(about = "Prints Cache Statistics")]
	Cache {
		#[clap(help = "Clears the Cache", short, long)]
		clear: bool,
	},

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

	handle_command(args.command);
}
