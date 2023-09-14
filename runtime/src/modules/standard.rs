/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::{Context, Object};
use ion::flags::PropertyFlags;
use ion::module::{Module, ModuleRequest};

pub trait StandardModules {
	fn init<'cx: 'o, 'o>(self, cx: &'cx Context, global: &mut Object<'o>) -> bool;

	fn init_globals<'cx: 'o, 'o>(self, cx: &'cx Context, global: &mut Object<'o>) -> bool;
}

impl StandardModules for () {
	fn init<'cx: 'o, 'o>(self, _: &'cx Context, _: &mut Object<'o>) -> bool {
		true
	}

	fn init_globals<'cx: 'o, 'o>(self, _: &'cx Context, _: &mut Object<'o>) -> bool {
		true
	}
}

pub trait NativeModule {
	const NAME: &'static str;
	const SOURCE: &'static str;

	fn module<'cx>(cx: &'cx Context) -> Option<Object<'cx>>;
}

impl<M: NativeModule> StandardModules for M {
	fn init<'cx: 'o, 'o>(self, cx: &'cx Context, global: &mut Object<'o>) -> bool {
		init_module::<M>(cx, global)
	}

	fn init_globals<'cx: 'o, 'o>(self, cx: &'cx Context, global: &mut Object<'o>) -> bool {
		init_global_module::<M>(cx, global)
	}
}

// TODO: Remove JS Wrapper, Stop Global Scope Pollution, Use CreateEmptyModule and AddModuleExport
//TODO: Waiting on https://bugzilla.mozilla.org/show_bug.cgi?id=1722802
pub fn init_module<'cx: 'o, 'o, M: NativeModule>(cx: &'cx Context, global: &mut Object<'o>) -> bool {
	let internal = format!("______{}Internal______", M::NAME);
	let module = M::module(cx);

	if let Some(module) = module {
		if global.define_as(cx, &internal, &module, PropertyFlags::CONSTANT) {
			let (module, _) = Module::compile(cx, M::NAME, None, M::SOURCE).unwrap();
			let loader = unsafe { &mut (*cx.get_inner_data()).module_loader };
			return loader.as_mut().is_some_and(|loader| {
				let request = ModuleRequest::new(cx, M::NAME);
				loader.register(cx, module.0.handle().get(), &request);
				true
			});
		}
	}
	false
}

pub fn init_global_module<'cx: 'o, 'o, M: NativeModule>(cx: &'cx Context, global: &mut Object<'o>) -> bool {
	let module = M::module(cx);

	if let Some(module) = module {
		global.define_as(cx, M::NAME, &module, PropertyFlags::CONSTANT_ENUMERATED)
	} else {
		false
	}
}
