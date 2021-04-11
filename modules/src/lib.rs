// #[macro_use]
// extern crate macro_rules_attributes;
#[macro_use]
extern crate mozjs;

use mozjs::jsapi::*;

mod fs;

pub fn init_modules(cx: *mut JSContext, global: *mut JSObject) -> bool {
	fs::fs::init_fs(cx, global)
}
