use ion::{js_fn, Object};

#[js_fn]
pub fn object(_object: Object) {}

#[js_fn]
pub fn object_optional(_object: Option<Object>) {}

#[js_fn]
pub fn object_vec(_objects: Vec<Object>) {}
