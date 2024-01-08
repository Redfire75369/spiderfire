use ion::{Context, Function, js_fn, Object, Promise, Value};
use ion::function::{Enforce, Rest, Strict};

#[allow(clippy::too_many_arguments)]
#[js_fn]
pub fn many_inputs(
	_cx: &Context, #[ion(this)] _this: &Object, Enforce(_integer): Enforce<i8>, Strict(_boolean): Strict<bool>,
	#[ion(convert = ())] Strict(_string): Strict<String>, _function: Function, _promise: Promise,
	Rest(_values): Rest<Value>,
) {
}
