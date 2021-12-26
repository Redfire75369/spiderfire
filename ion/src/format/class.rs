/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::ffi::CStr;

use colored::Colorize;
use mozjs::rust::get_object_class;

use crate::format::config::FormatConfig;
use crate::format::object::format_object_raw;
use crate::IonContext;
use crate::objects::object::IonObject;

pub unsafe fn format_class_object(cx: IonContext, cfg: FormatConfig, object: IonObject) -> String {
	let class = get_object_class(object.raw());
	let name = CStr::from_ptr((*class).name).to_str().unwrap();

	let string = format_object_raw(cx, cfg, object);
	format!("{} {}", name.color(cfg.colors.object), string)
}
