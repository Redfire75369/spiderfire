use ion::{Arguments, Context, js_fn};

#[js_fn]
fn context(_cx: &Context) {}

#[js_fn]
fn arguments(_args: &mut Arguments) {}

#[js_fn]
fn context_arguments(_cx: &Context, _args: &mut Arguments) {}
