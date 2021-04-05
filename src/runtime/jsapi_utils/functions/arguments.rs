use mozjs::jsapi::*;
use ::std::ops::RangeBounds;

pub(crate) struct Arguments {
	pub(crate) values: Vec<Handle<Value>>,
	call_args: CallArgs,
}

impl Arguments {
	pub(crate) unsafe fn new(argc: u32, vp: *mut Value) -> Arguments {
		let call_args = CallArgs::from_vp(vp, argc);

		let values: Vec<_> = (0..argc).map(|i| call_args.get(i)).collect();

		Arguments { values, call_args }
	}

	pub(crate) fn len(&self) -> usize {
		self.values.len()
	}

	#[allow(dead_code)]
	pub(crate) fn handle(&self, index: usize) -> Option<Handle<Value>> {
		if self.len() > index + 1 {
			return Some(self.values[index]);
		}
		None
	}

	pub(crate) fn value(&self, index: usize) -> Option<Value> {
		if self.len() > index + 1 {
			return Some(self.values[index].get());
		}
		None
	}

	pub(crate) fn range<R: Iterator<Item = usize> + RangeBounds<usize>>(&self, range: R) -> Vec<Value> {
		range.filter_map(|index| self.value(index)).collect::<Vec<_>>()
	}

	pub(crate) fn range_full(&self) -> Vec<Value> {
		self.values.iter().map(|value| value.get()).collect::<Vec<_>>()
	}

	pub(crate) fn rval(&self) -> MutableHandle<Value> {
		self.call_args.rval()
	}
}
