/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

mod decoder;
mod encoder;

pub use decoder::TextDecoder;
pub use encoder::TextEncoder;
use ion::{ClassInitialiser, Context, Object};

pub fn define(cx: &Context, global: &mut Object) -> bool {
	TextDecoder::init_class(cx, global);
	TextEncoder::init_class(cx, global);
	true
}
