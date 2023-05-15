/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub use class::*;

#[js_class]
mod class {
	use std::mem::transmute;
	use mozjs::jsapi::{Heap, JSObject, JSTracer};
	use mozjs::gc::Traceable;
	use ion::{ClassInitialiser, Context, Error, ErrorKind, Object, Result};
	use url::Url;
	use crate::url::URL;

	#[ion(into_value)]
	pub struct URLSearchParams {
		object: Box<Heap<*mut JSObject>>,
		url: &'static mut Url,
	}

	// TODO: Allow URLSearchParams to be formed with just a string of query pairs
	// TODO: Implement URLSearchParams.prototype.set and URLSearchParams.prototype.delete
	impl URLSearchParams {
		#[allow(unused_mut)]
		#[ion(constructor)]
		pub fn constructor(cx: &Context, mut object: Object) -> Result<URLSearchParams> {
			if URL::instance_of(cx, &object, None) {
				let url = unsafe { transmute(URL::get_private(&mut object)) };
				let object = Heap::boxed(**object);
				Ok(URLSearchParams { object, url })
			} else {
				Err(Error::new("Expected URL", ErrorKind::Type))
			}
		}

		pub fn append(&mut self, name: String, value: String) {
			self.url.query_pairs_mut().append_pair(&name, &value);
		}

		pub fn get(&self, key: String) -> Option<String> {
			self.url.query_pairs().into_owned().find(|(k, _)| k == &key).map(|(_, v)| v)
		}

		pub fn getAll(&self, key: String) -> Vec<String> {
			self.url.query_pairs().into_owned().filter(|(k, _)| k == &key).map(|(_, v)| v).collect()
		}

		pub fn has(&self, key: String, value: Option<String>) -> bool {
			if let Some(value) = value {
				self.url.query_pairs().into_owned().any(|(k, v)| k == key && v == value)
			} else {
				self.url.query_pairs().into_owned().any(|(k, _)| k == key)
			}
		}

		pub fn size(&self) -> i32 {
			self.url.query_pairs().count() as i32
		}
	}

	unsafe impl Traceable for URLSearchParams {
		unsafe fn trace(&self, trc: *mut JSTracer) {
			self.object.trace(trc);
		}
	}
}
