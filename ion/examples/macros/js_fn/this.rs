use ion::{Object, js_fn};

#[js_fn]
fn this_object(#[ion(this)] _this: &Object) {}
