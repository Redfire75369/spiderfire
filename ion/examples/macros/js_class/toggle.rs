use class::Toggle;
use ion::js_class;

#[js_class]
mod class {
	use ion::{Context, Function, Object, Result, Value};
	use ion::conversions::{ConversionBehavior, FromValue};
	use ion::symbol::WellKnownSymbolCode;

	#[derive(Clone, Default)]
	#[ion(from_value, to_value)]
	pub struct Toggle {
		#[ion(skip)]
		pub toggle: bool,
		#[ion(name = "toggles", alias = ["switches"], readonly)]
		pub toggled: i32,
		#[ion(convert = ConversionBehavior::EnforceRange)]
		pub arbitrary: u8,
	}

	impl Toggle {
		pub const DEFAULT_TOGGLED: i32 = 0;

		#[ion(constructor)]
		pub fn constructor() -> Toggle {
			Toggle::default()
		}

		#[ion(skip)]
		pub fn reset(&mut self) -> bool {
			self.toggled = 0;
			true
		}

		#[ion(name = "callback", alias = ["if_toggled", "if_switched"])]
		pub unsafe fn if_toggled(&self, cx: &Context, function: Function) -> Result<String> {
			let value = function.call(cx, &Object::null(cx), &[Value::i32(cx, self.toggled)]).unwrap();
			String::from_value(cx, &value, false, ())
		}

		#[ion(get, alias = ["switch"])]
		pub fn get_toggle(&self) -> i32 {
			self.toggled * self.toggle as i32
		}

		#[ion(set)]
		pub fn set_toggle(&mut self, toggle: bool) -> bool {
			let toggled = self.toggled;
			if !self.reset() {
				return false;
			}
			self.toggled = toggled + toggle as i32;
			self.toggle = toggle;
			toggle
		}
	}
}

fn ensure_callable() {
	let mut toggle = Toggle::default();
	toggle.get_toggle();
	toggle.set_toggle(true);
	toggle.reset();
}
