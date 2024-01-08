use ion::{Context, Function, js_fn, Object, Promise, Value};
use ion::conversions::ConversionBehavior;
use ion::function::{Rest, Strict};

#[allow(clippy::too_many_arguments)]
#[js_fn]
pub fn many_inputs(
	_cx: &Context, #[ion(this)] _this: &Object, #[ion(convert = ConversionBehavior::EnforceRange)] _integer: i8,
	Strict(_boolean): Strict<bool>, #[ion(convert = ())] Strict(_string): Strict<String>, _function: Function,
	_promise: Promise, Rest(_values): Rest<Value>,
) {
}
