use ion::conversions::ConversionBehavior;
use ion::js_fn;

#[js_fn]
pub fn integer(#[ion(convert = ConversionBehavior::Clamp)] _integer: u8) {}

#[js_fn]
pub fn integer_optional(#[ion(convert = ConversionBehavior::Clamp)] _integer: Option<i16>) {}

#[js_fn]
pub fn integer_vec(#[ion(convert = ConversionBehavior::Clamp)] _integers: Vec<u32>) {}
