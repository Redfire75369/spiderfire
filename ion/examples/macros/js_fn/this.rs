use ion::{js_fn, Object};

#[js_fn]
fn this_object(#[ion(this)] _this: &Object) {}
