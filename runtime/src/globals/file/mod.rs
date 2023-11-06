/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use blob::Blob;
pub use blob::buffer_source_to_bytes;
use ion::{ClassDefinition, Context, Object};

mod blob;

pub fn define(cx: &Context, object: &mut Object) -> bool {
	Blob::init_class(cx, object).0
}
