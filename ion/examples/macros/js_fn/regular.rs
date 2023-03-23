use ion::js_fn;

#[js_fn]
fn regular(_string: String) {}

#[js_fn]
fn regular_optional(_string: Option<String>) {}

#[js_fn]
fn regular_vec(_strings: Vec<String>) {}
