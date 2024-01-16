/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;

use mozjs::jsapi::{Heap, JSObject};
use url::Url;

use ion::{ClassDefinition, Context, Error, Local, Object, Result};
use ion::class::Reflector;
use ion::function::{Enforce, Opt};
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
pub struct URL {
	reflector: Reflector,
	#[trace(no_trace)]
	pub(crate) url: Url,
	search_params: Box<Heap<*mut JSObject>>,
}

#[js_class]
impl URL {
	#[ion(constructor)]
	pub fn constructor(#[ion(this)] this: &Object, cx: &Context, input: String, Opt(base): Opt<String>) -> Result<URL> {
		let base = base.as_ref().and_then(|base| Url::parse(base).ok());
		let url = Url::options()
			.base_url(base.as_ref())
			.parse(&input)
			.map_err(|error| Error::new(&error.to_string(), None))?;

		let search_params = Box::new(URLSearchParams::new(url.query_pairs().into_owned().collect()));
		search_params.url.as_ref().unwrap().set(this.handle().get());
		let search_params = Heap::boxed(URLSearchParams::new_object(cx, search_params));

		Ok(URL {
			reflector: Reflector::default(),
			url,
			search_params,
		})
	}

	#[ion(name = "canParse")]
	pub fn can_parse(input: String, Opt(base): Opt<String>) -> bool {
		let base = base.as_ref().and_then(|base| Url::parse(base).ok());
		Url::options().base_url(base.as_ref()).parse(&input).is_ok()
	}

	pub fn format(&self, Opt(options): Opt<FormatOptions>) -> Result<String> {
		let mut url = self.url.clone();

		let options = options.unwrap_or_default();
		if !options.auth {
			url.set_username("").map_err(|_| Error::new("Invalid Url", None))?;
		}
		if !options.fragment {
			url.set_fragment(None);
		}
		if !options.search {
			url.set_query(None);
		}

		Ok(url.to_string())
	}

	#[ion(name = "toString", alias = ["toJSON"])]
	#[allow(clippy::inherent_to_string)]
	pub fn to_string(&self) -> String {
		self.url.to_string()
	}

	#[ion(get)]
	pub fn get_href(&self) -> String {
		self.url.to_string()
	}

	#[ion(set)]
	pub fn set_href(&mut self, cx: &Context, input: String) -> Result<()> {
		match Url::parse(&input) {
			Ok(url) => {
				let search_params = Object::from(unsafe { Local::from_heap(&self.search_params) });
				let search_params = URLSearchParams::get_mut_private(cx, &search_params)?;
				search_params.set_pairs(url.query_pairs().into_owned().collect());
				self.url = url;
				Ok(())
			}
			Err(error) => Err(Error::new(&error.to_string(), None)),
		}
	}

	#[ion(get)]
	pub fn get_protocol(&self) -> String {
		String::from(self.url.scheme())
	}

	#[ion(set)]
	pub fn set_protocol(&mut self, protocol: String) -> Result<()> {
		self.url.set_scheme(&protocol).map_err(|_| Error::new("Invalid Protocol", None))
	}

	#[ion(get)]
	pub fn get_host(&self) -> Option<String> {
		self.url.host_str().map(|host| {
			if let Some(port) = self.url.port() {
				format!("{}:{}", host, port)
			} else {
				String::from(host)
			}
		})
	}

	#[ion(set)]
	pub fn set_host(&mut self, Opt(host): Opt<String>) -> Result<()> {
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

			self.url.set_host(Some(host))?;

			self.url.set_port(port).map_err(|_| Error::new("Invalid Url", None))?;
		} else {
			self.url.set_host(None)?;
			self.url.set_port(None).map_err(|_| Error::new("Invalid Url", None))?;
		}
		Ok(())
	}

	#[ion(get)]
	pub fn get_hostname(&self) -> Option<String> {
		self.url.host_str().map(String::from)
	}

	#[ion(set)]
	pub fn set_hostname(&mut self, Opt(hostname): Opt<String>) -> Result<()> {
		self.url
			.set_host(hostname.as_deref())
			.map_err(|error| Error::new(&error.to_string(), None))
	}

	#[ion(get)]
	pub fn get_origin(&self) -> String {
		self.url.origin().ascii_serialization()
	}

	#[ion(get)]
	pub fn get_port(&self) -> Option<u16> {
		self.url.port_or_known_default()
	}

	#[ion(set)]
	pub fn set_port(&mut self, Opt(port): Opt<Enforce<u16>>) -> Result<()> {
		self.url.set_port(port.map(|p| p.0)).map_err(|_| Error::new("Invalid Port", None))
	}

	#[ion(get)]
	pub fn get_pathname(&self) -> String {
		String::from(self.url.path())
	}

	#[ion(set)]
	pub fn set_pathname(&mut self, path: String) -> Result<()> {
		self.url.set_path(&path);
		Ok(())
	}

	#[ion(get)]
	pub fn get_username(&self) -> String {
		String::from(self.url.username())
	}

	#[ion(set)]
	pub fn set_username(&mut self, username: String) -> Result<()> {
		self.url.set_username(&username).map_err(|_| Error::new("Invalid Url", None))
	}

	#[ion(get)]
	pub fn get_password(&self) -> Option<String> {
		self.url.password().map(String::from)
	}

	#[ion(set)]
	pub fn set_password(&mut self, Opt(password): Opt<String>) -> Result<()> {
		self.url.set_password(password.as_deref()).map_err(|_| Error::new("Invalid Url", None))
	}

	#[ion(get)]
	pub fn get_search(&self) -> Option<String> {
		self.url.query().map(String::from)
	}

	#[ion(set)]
	pub fn set_search(&mut self, Opt(search): Opt<String>) {
		self.url.set_query(search.as_deref());
	}

	#[ion(get)]
	pub fn get_hash(&self) -> Option<String> {
		self.url.fragment().map(String::from)
	}

	#[ion(set)]
	pub fn set_hash(&mut self, Opt(hash): Opt<String>) {
		self.url.set_fragment(hash.as_deref());
	}

	#[ion(get)]
	pub fn get_search_params(&self) -> *mut JSObject {
		self.search_params.get()
	}
}

pub fn define(cx: &Context, global: &Object) -> bool {
	URL::init_class(cx, global).0 && URLSearchParams::init_class(cx, global).0
}
