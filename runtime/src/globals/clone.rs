/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::{CloneDataPolicy, JSStructuredCloneCallbacks, StructuredCloneScope};

use ion::{Context, Object, ResultExc, Value};
use ion::clone::StructuredCloneBuffer;
use ion::flags::PropertyFlags;
use ion::function::Opt;

pub static STRUCTURED_CLONE_CALLBACKS: JSStructuredCloneCallbacks = JSStructuredCloneCallbacks {
	read: None,
	write: None,
	reportError: None,
	readTransfer: None,
	writeTransfer: None,
	freeTransfer: None,
	canTransfer: None,
	sabCloned: None,
};

#[derive(FromValue)]
struct StructuredCloneOptions<'cx> {
	transfer: Vec<Object<'cx>>,
}

#[js_fn]
fn structured_clone<'cx>(
	cx: &'cx Context, data: Value<'cx>, Opt(options): Opt<StructuredCloneOptions<'cx>>,
) -> ResultExc<Value<'cx>> {
	let transfer = options.map(|o| o.transfer);
	let policy = CloneDataPolicy {
		allowIntraClusterClonableSharedObjects_: false,
		allowSharedMemoryObjects_: true,
	};

	let mut buffer = StructuredCloneBuffer::new(StructuredCloneScope::SameProcess, &STRUCTURED_CLONE_CALLBACKS);
	buffer.write(cx, &data, transfer, &policy)?;
	buffer.read(cx, &policy)
}

pub fn define(cx: &Context, global: &Object) -> bool {
	!global
		.define_method(
			cx,
			"structuredClone",
			structured_clone,
			1,
			PropertyFlags::CONSTANT_ENUMERATED,
		)
		.handle()
		.is_null()
}
