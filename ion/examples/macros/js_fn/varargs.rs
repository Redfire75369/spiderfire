use ion::{js_fn, Object};
use ion::conversions::ConversionBehavior;

#[js_fn]
pub fn varargs(#[ion(varargs)] _strings: Vec<String>) {}

#[js_fn]
pub fn varargs_integer(#[ion(varargs, convert = ConversionBehavior::EnforceRange)] _integers: Vec<i64>) {}

#[js_fn]
pub fn varargs_object(#[ion(varargs)] _objects: Vec<Object>) {}
