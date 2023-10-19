use ion::{Context, Function, js_fn, Object, Promise, Value};
use ion::conversions::ConversionBehavior;

#[allow(clippy::too_many_arguments)]
#[js_fn]
pub fn many_inputs(
	_cx: &Context, #[ion(this)] _this: &Object, #[ion(convert = ConversionBehavior::EnforceRange)] _integer: i8, #[ion(strict)] _boolean: bool,
	#[ion(convert = (), strict)] _string: String, _function: Function, _promise: Promise, #[ion(varargs)] _values: Vec<Value>,
) {
}
