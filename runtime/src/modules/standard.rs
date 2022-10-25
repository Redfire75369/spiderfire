/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::{Context, Object};
use ion::flags::PropertyFlags;

use crate::modules::Module;

pub trait StandardModules {
	fn init<'cx>(cx: &'cx Context, global: &mut Object<'cx>) -> bool;

	fn init_globals<'cx>(cx: &'cx Context, global: &mut Object<'cx>) -> bool;
}

impl StandardModules for () {
	fn init(_: &Context, _: &mut Object) -> bool {
		true
	}

	fn init_globals(_: &Context, _: &mut Object) -> bool {
		true
	}
}

pub trait NativeModule {
	const NAME: &'static str;
	const SOURCE: &'static str;

	fn module<'cx>(cx: &'cx Context) -> Option<Object<'cx>>;
}

impl<M: NativeModule> StandardModules for M {
	fn init<'cx>(cx: &'cx Context, global: &mut Object<'cx>) -> bool {
		init_module::<M>(cx, global)
	}

	fn init_globals<'cx>(cx: &'cx Context, global: &mut Object<'cx>) -> bool {
		init_global_module::<M>(cx, global)
	}
}

pub fn init_global_module<'cx, M: NativeModule>(cx: &'cx Context, global: &mut Object<'cx>) -> bool {
	let module = M::module(cx);

	if let Some(module) = module {
		global.define_as(cx, M::NAME, module, PropertyFlags::CONSTANT_ENUMERATED)
	} else {
		false
	}
}

/*
 * TODO: Remove JS Wrapper, Stop Global Scope Pollution, Use CreateEmptyModule and AddModuleExport
 * TODO: Waiting on https://bugzilla.mozilla.org/show_bug.cgi?id=1722802
 */
pub fn init_module<'cx, M: NativeModule>(cx: &'cx Context, global: &mut Object<'cx>) -> bool {
	let internal = format!("______{}Internal______", M::NAME);
	let module = M::module(cx);

	if let Some(module) = module {
		if global.define_as(cx, &internal, module, PropertyFlags::CONSTANT) {
			let module = Module::compile(cx, M::NAME, None, M::SOURCE).unwrap();
			return module.0.register(M::NAME);
		}
	}
	false
}
