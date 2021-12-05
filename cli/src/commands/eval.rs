/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use runtime::globals::{init_globals, new_global};
use runtime::microtask_queue::init_microtask_queue;
use runtime::new_runtime;

use crate::evaluate::eval_inline;

pub fn eval_source(source: &str) {
	let (_engine, rt) = new_runtime();
	let (global, _ac) = new_global(rt.cx());

	init_globals(rt.cx(), global);
	let queue = init_microtask_queue(rt.cx());

	eval_inline(&rt, &queue, source);
}
