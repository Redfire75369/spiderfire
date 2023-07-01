/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::{ClassInitialiser, Context, Object};
pub use url_class::URL;
pub use search_params::URLSearchParams;

mod search_params;

#[derive(Default, FromValue)]
pub struct FormatOptions {
	#[ion(default)]
	auth: bool,
	#[ion(default)]
	fragment: bool,
	#[ion(default)]
	search: bool,
}

#[js_class]
mod url_class {
	use std::cell::RefCell;
	use std::cmp::Ordering;
	use std::rc::Rc;

	use mozjs::conversions::ConversionBehavior::EnforceRange;
	use mozjs::gc::Traceable;
	use mozjs::jsapi::{Heap, JSObject, JSTracer};
	use url::Url;

	use ion::{Context, Error, Result};
	use ion::ClassInitialiser;

	use super::FormatOptions;
	use super::search_params::URLSearchParams;

	#[allow(clippy::upper_case_acronyms)]
	#[ion(from_value, to_value)]
	pub struct URL {
		url: Rc<RefCell<Url>>,
		search_params: Box<Heap<*mut JSObject>>,
	}

	impl URL {
		#[ion(constructor)]
		pub fn constructor(cx: &Context, input: String, base: Option<String>) -> Result<URL> {
			let options = Url::options();
			let base = base.as_ref().and_then(|base| Url::parse(base).ok());
			options.base_url(base.as_ref());
			let url = options.parse(&input).map_err(|error| Error::new(&error.to_string(), None))?;
			let url = Rc::new(RefCell::new(url));

			let search_params = URLSearchParams::from_url(Rc::clone(&url));
			let search_params = Heap::boxed(URLSearchParams::new_object(cx, search_params));

			Ok(URL { url, search_params })
		}

		pub fn canParse(input: String, base: Option<String>) -> bool {
			let options = Url::options();
			let base = base.as_ref().and_then(|base| Url::parse(base).ok());
			options.base_url(base.as_ref());
			options.parse(&input).is_ok()
		}

		pub fn format(&self, options: Option<FormatOptions>) -> Result<String> {
			let mut url = self.url.borrow().clone();

			let options = options.unwrap_or_default();
			if !options.auth {
				url.set_username("").map_err(|_| Error::new("Invalid URL", None))?;
			}
			if !options.fragment {
				url.set_fragment(None);
			}
			if !options.search {
				url.set_query(None);
			}

			Ok(url.to_string())
		}

		#[ion(alias = ["toJSON"])]
		pub fn toString(&self) -> String {
			self.url.borrow().to_string()
		}

		#[ion(get)]
		pub fn get_href(&self) -> String {
			self.url.borrow().to_string()
		}

		#[ion(set)]
		pub fn set_href(&mut self, cx: &Context, input: String) -> Result<()> {
			match Url::parse(&input) {
				Ok(url) => {
					self.url = Rc::new(RefCell::new(url));
					let search_params = URLSearchParams::from_url(Rc::clone(&self.url));
					self.search_params = Heap::boxed(URLSearchParams::new_object(cx, search_params));
					Ok(())
				}
				Err(error) => Err(Error::new(&error.to_string(), None)),
			}
		}

		#[ion(get)]
		pub fn get_protocol(&self) -> String {
			String::from(self.url.borrow().scheme())
		}

		#[ion(set)]
		pub fn set_protocol(&mut self, protocol: String) -> Result<()> {
			self.url
				.borrow_mut()
				.set_scheme(&protocol)
				.map_err(|_| Error::new("Invalid Protocol", None))
		}

		#[ion(get)]
		pub fn get_host(&self) -> Option<String> {
			self.url.borrow().host_str().map(|host| {
				if let Some(port) = self.url.borrow().port() {
					format!("{}:{}", host, port)
				} else {
					String::from(host)
				}
			})
		}

		#[ion(set)]
		pub fn set_host(&mut self, host: Option<String>) -> Result<()> {
			let mut url = self.url.borrow_mut();
			if let Some(host) = host {
				let segments: Vec<&str> = host.split(':').collect();
				let (host, port) = match segments.len().cmp(&2) {
					Ordering::Less => Ok((segments[0], None)),
					Ordering::Greater => Err(Error::new("Invalid Host", None)),
					Ordering::Equal => {
						let port = match segments[1].parse::<u16>() {
							Ok(port) => Ok(port),
							Err(error) => Err(Error::new(&error.to_string(), None)),
						}?;
						Ok((segments[0], Some(port)))
					}
				}?;

				url.set_host(Some(host))?;

				url.set_port(port).map_err(|_| Error::new("Invalid URL", None))?;
			} else {
				url.set_host(None)?;
				url.set_port(None).map_err(|_| Error::new("Invalid URL", None))?;
			}
			Ok(())
		}

		#[ion(get)]
		pub fn get_hostname(&self) -> Option<String> {
			self.url.borrow().host_str().map(String::from)
		}

		#[ion(set)]
		pub fn set_hostname(&mut self, hostname: Option<String>) -> Result<()> {
			self.url
				.borrow_mut()
				.set_host(hostname.as_deref())
				.map_err(|error| Error::new(&error.to_string(), None))
		}

		#[ion(get)]
		pub fn get_origin(&self) -> String {
			self.url.borrow().origin().ascii_serialization()
		}

		#[ion(get)]
		pub fn get_port(&self) -> Option<u16> {
			self.url.borrow().port_or_known_default()
		}

		#[ion(set)]
		pub fn set_port(&mut self, #[ion(convert = EnforceRange)] port: Option<u16>) -> Result<()> {
			self.url.borrow_mut().set_port(port).map_err(|_| Error::new("Invalid Port", None))
		}

		#[ion(get)]
		pub fn get_path(&self) -> String {
			String::from(self.url.borrow().path())
		}

		#[ion(set)]
		pub fn set_path(&mut self, path: String) -> Result<()> {
			self.url.borrow_mut().set_path(&path);
			Ok(())
		}

		#[ion(get)]
		pub fn get_username(&self) -> String {
			String::from(self.url.borrow().username())
		}

		#[ion(set)]
		pub fn set_username(&mut self, username: String) -> Result<()> {
			self.url.borrow_mut().set_username(&username).map_err(|_| Error::new("Invalid URL", None))
		}

		#[ion(get)]
		pub fn get_password(&self) -> Option<String> {
			self.url.borrow().password().map(String::from)
		}

		#[ion(set)]
		pub fn set_password(&mut self, password: Option<String>) -> Result<()> {
			self.url
				.borrow_mut()
				.set_password(password.as_deref())
				.map_err(|_| Error::new("Invalid URL", None))
		}

		#[ion(get)]
		pub fn get_search(&self) -> Option<String> {
			self.url.borrow().query().map(String::from)
		}

		#[ion(set)]
		pub fn set_search(&mut self, search: Option<String>) {
			self.url.borrow_mut().set_query(search.as_deref());
		}

		#[ion(get)]
		pub fn get_hash(&self) -> Option<String> {
			self.url.borrow().fragment().map(String::from)
		}

		#[ion(set)]
		pub fn set_hash(&mut self, hash: Option<String>) {
			self.url.borrow_mut().set_fragment(hash.as_deref());
		}

		#[ion(get)]
		pub fn get_search_params(&self) -> *mut JSObject {
			self.search_params.get()
		}
	}

	impl Clone for URL {
		fn clone(&self) -> URL {
			URL {
				url: Rc::clone(&self.url),
				search_params: Heap::boxed(self.search_params.get()),
			}
		}
	}

	unsafe impl Traceable for URL {
		#[inline]
		unsafe fn trace(&self, trc: *mut JSTracer) {
			self.search_params.trace(trc);
		}
	}
}

pub fn define(cx: &Context, global: &mut Object) -> bool {
	URL::init_class(cx, global).0 && URLSearchParams::init_class(cx, global).0
}
