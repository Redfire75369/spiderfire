/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use clap::{App, Arg, SubCommand};

use runtime::config::{Config, CONFIG, LogLevel};
use structopt::StructOpt;
use crate::commands::{repl, run};
use crate::commands::eval;

mod commands;
pub mod evaluate;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "spiderfire",
    about = "Javascript Runtime"
)]
pub enum Cli {
    #[structopt(about="Evaluated a line of Javascript")]
    Eval {
        #[structopt(required(true), about="Line of Javascript to be evaluated")]
        source: String
    },

    #[structopt(about="Starts a Javascript Shell")]
    Repl,

    #[structopt(about="Runs a javascript file")]
    Run {
        #[structopt(about="The Javascript file to run. Default: 'main.js'", required(false), default_value="main.js")]
        path: String,

        #[structopt(about="Sets loggin level, Default: ERROR",required(false))]
        log_level: String,

        #[structopt(about="Sets logging level to DEBUG.", short)]
        debug: bool,

        #[structopt(about="Disables ES Modules Features", short)]
        script: bool
    }
}

fn main() {
    let args = Cli::from_args();

    match args {
        Eval { source } => {
             //CONFIG
				//.set(Config::default().log_level(LogLevel::Debug).script(true))
				//.expect("Config Initialisation Failed");
			//eval::eval_source(source);
            println!("{}", source);
        }
        Repl | _ => {
            //CONFIG
				//.set(Config::default().log_level(LogLevel::Debug).script(true))
				//.expect("Config Initialisation Failed");
			//repl::start_repl();
            println!("REPL!");
        }
        Run { path, log_level, debug, script } => {
			let mut log_lev = LogLevel::Error;

            if debug {
				log_lev = LogLevel::Debug
			} else  {

			    match log_level.to_uppercase().as_str() {
                    "NONE" => log_lev = LogLevel::None,
				    "INFO" => log_lev = LogLevel::Info,
				    "WARN" => log_lev = LogLevel::Warn,
				    "ERROR" => log_lev = LogLevel::Error,
				    "DEBUG" => log_lev = LogLevel::Debug,
				    _ => panic!("Invalid Logging Level")
                }
            }
			//CONFIG
				//.set(Config::default().log_level(log_level).script(script))
				//.expect("Config Initialisation Failed");
			//run::run(path);

            match log_lev {
                LogLevel::None => println!("none"),
                LogLevel::Info => println!("info"),
                LogLevel::Warn => println!("Warn"),
                LogLevel::Error => println!("Error"),
                LogLevel::Debug => println!("Debug")
            }
        }

    }

}
