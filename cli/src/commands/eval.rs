/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::rust::JSEngine;

use modules::Modules;
use runtime::RuntimeBuilder;

use crate::evaluate::eval_inline;

pub fn eval_source(source: &str) {
	let engine = JSEngine::init().unwrap();
	let rt = RuntimeBuilder::<Modules>::new()
		.microtask_queue()
		.macrotask_queue()
		.standard_modules()
		.build(engine.handle());
	eval_inline(&rt, source);
}
