/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use runtime::globals::new_global;
use runtime::new_runtime;

use crate::evaluate::eval_inline;

pub fn eval_source(source: &str) {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	eval_inline(&rt, global, source);
}
