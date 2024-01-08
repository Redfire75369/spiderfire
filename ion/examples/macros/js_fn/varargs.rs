use ion::{js_fn, Object};
use ion::conversions::ConversionBehavior;
use ion::functions::Rest;

#[js_fn]
pub fn varargs(Rest(_strings): Rest<String>) {}

#[js_fn]
pub fn varargs_integer(#[ion(convert = ConversionBehavior::EnforceRange)] Rest(_integers): Rest<i64>) {}

#[js_fn]
pub fn varargs_object(Rest(_objects): Rest<Object>) {}
