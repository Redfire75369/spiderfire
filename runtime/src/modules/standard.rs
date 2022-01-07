/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::flags::PropertyFlags;
use ion::IonContext;
use ion::objects::object::IonObject;

use crate::modules::IonModule;

pub trait StandardModules {
	fn init(cx: IonContext, global: &mut IonObject) -> bool;

	fn init_globals(cx: IonContext, global: &mut IonObject) -> bool;
}

impl StandardModules for () {
	fn init(_: IonContext, _: &mut IonObject) -> bool {
		true
	}

	fn init_globals(_: IonContext, _: &mut IonObject) -> bool {
		true
	}
}

pub trait Module {
	const NAME: &'static str;
	const SOURCE: &'static str;

	unsafe fn module(cx: IonContext) -> Option<IonObject>;
}

impl<M: Module> StandardModules for M {
	fn init(cx: IonContext, global: &mut IonObject) -> bool {
		unsafe { init_module::<M>(cx, global) }
	}

	fn init_globals(cx: IonContext, global: &mut IonObject) -> bool {
		unsafe { init_global_module::<M>(cx, global) }
	}
}

pub unsafe fn init_global_module<M: Module>(cx: IonContext, global: &mut IonObject) -> bool {
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
pub unsafe fn init_module<M: Module>(cx: IonContext, global: &mut IonObject) -> bool {
	let internal = format!("______{}Internal______", M::NAME);
	let module = M::module(cx);

	if let Some(module) = module {
		if global.define_as(cx, &internal, module, PropertyFlags::CONSTANT) {
			let module = IonModule::compile(cx, M::NAME, None, M::SOURCE).unwrap();
			return module.register(M::NAME);
		}
	}
	false
}
