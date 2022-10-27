use ion::{Array, js_fn, Object, Value};

#[js_fn]
fn this_object(#[ion(this)] _this: Object) {}

#[js_fn]
fn this_array(#[ion(this)] _this: Array) {}

#[js_fn]
fn this_value(#[ion(this)] _this: Value) {}
