use ion::{js_fn, Arguments, Context};

#[js_fn]
pub fn context(_cx: &Context) {}

#[js_fn]
pub fn arguments(_args: &mut Arguments) {}

#[js_fn]
pub fn context_arguments(_cx: &Context, _args: &mut Arguments) {}
