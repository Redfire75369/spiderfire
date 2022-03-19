/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use rustyline::{Config, Result};
use rustyline::config::Builder;
use rustyline::validate::{MatchingBracketValidator, ValidationContext, ValidationResult, Validator};

#[derive(Completer, Helper, Hinter, Highlighter)]
pub struct ReplHelper;

impl Validator for ReplHelper {
	fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult> {
		MatchingBracketValidator::new().validate(ctx)
	}
}

pub fn rustyline_config() -> Config {
	let builder = Builder::new();
	builder.tab_stop(4).build()
}
