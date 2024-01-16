use ion::{js_fn, Object};
use ion::function::{Enforce, Rest};

#[js_fn]
pub fn varargs(Rest(_strings): Rest<String>) {}

#[js_fn]
pub fn varargs_integer(Rest(_integers): Rest<Enforce<i64>>) {}

#[js_fn]
pub fn varargs_object(Rest(_objects): Rest<Object>) {}
