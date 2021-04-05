#[macro_use]
extern crate mozjs;

mod cli;
mod config;
mod modules;
mod runtime;
mod utils;

pub use crate::cli::{repl, run};
pub use crate::config::{Config, CONFIG};