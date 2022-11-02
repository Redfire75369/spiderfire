/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use runtime::config::Config;

use crate::evaluate::{eval_module, eval_script};

pub(crate) async fn run(path: &str) {
	if Config::global().script {
		eval_script(Path::new(path)).await;
	} else {
		eval_module(Path::new(path)).await;
	}
}
