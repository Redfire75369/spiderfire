use ion::function::Clamp;
use ion::js_fn;

#[js_fn]
pub fn integer(Clamp(_integer): Clamp<u8>) {}

#[js_fn]
pub fn integer_optional(_integer: Option<Clamp<i16>>) {}

#[js_fn]
pub fn integer_vec(_integers: Vec<Clamp<u32>>) {}
