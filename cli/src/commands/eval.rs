/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::rust::{JSEngine, Runtime};

use ion::Context;
use modules::Modules;
use runtime::RuntimeBuilder;

use crate::evaluate::eval_inline;

pub(crate) async fn eval_source(source: &str) {
	let engine = JSEngine::init().unwrap();
	let rt = Runtime::new(engine.handle());

	let cx = &mut Context::from_runtime(&rt);
	let rt = RuntimeBuilder::<(), _>::new()
		.microtask_queue()
		.macrotask_queue()
		.standard_modules(Modules)
		.build(cx);
	eval_inline(&rt, source).await;
}
