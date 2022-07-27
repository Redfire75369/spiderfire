/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::CStr;

use colored::Colorize;
use mozjs::rust::get_object_class;

use crate::{Context, Object};
use crate::format::Config;
use crate::format::object::format_object_raw;

/// Formats an [Object], along with the name of its constructor, as a [String] with the given [Config].
pub fn format_class_object(cx: Context, cfg: Config, object: Object) -> String {
	let class = unsafe { get_object_class(*object) };
	let name = unsafe { CStr::from_ptr((*class).name) }.to_str().unwrap();

	let string = format_object_raw(cx, cfg, object);
	format!("{} {}", name.color(cfg.colors.object), string)
}
