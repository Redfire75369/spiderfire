/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use idna::{domain_to_ascii, domain_to_ascii_strict, domain_to_unicode};
use mozjs::jsapi::JSFunctionSpec;

use ion::{Context, Object, Result};
use ion::ClassInitialiser;
use runtime::modules::NativeModule;

#[js_class]
mod class {
	use std::cmp::Ordering;

	use mozjs::conversions::ConversionBehavior::EnforceRange;
	use url::Url;

	use ion::{Context, Error, Object, Result};

	#[allow(clippy::upper_case_acronyms)]
	#[derive(Clone)]
	#[ion(from_value, to_value)]
	pub struct URL {
		url: Url,
	}

	impl URL {
		#[ion(constructor)]
		pub fn constructor(input: String, base: Option<String>) -> Result<URL> {
			let options = Url::options();
			let base = base.as_ref().and_then(|base| Url::parse(base).ok());
			options.base_url(base.as_ref());
			let url = options.parse(&input).map_err(|error| Error::new(&error.to_string(), None))?;
			Ok(URL { url })
		}

		pub fn origin(&self) -> String {
			self.url.origin().ascii_serialization()
		}

		pub fn toString(&self) -> String {
			self.url.to_string()
		}

		pub fn toJSON(&self) -> String {
			self.url.to_string()
		}

		pub fn format(&self, cx: &Context, options: Option<Object>) -> Result<String> {
			let mut url = self.url.clone();

			let auth = options.as_ref().and_then(|options| options.get_as(cx, "auth", true, ())).unwrap_or(true);
			let fragment = options
				.as_ref()
				.and_then(|options| options.get_as(cx, "fragment", true, ()))
				.unwrap_or(true);
			let search = options
				.as_ref()
				.and_then(|options| options.get_as(cx, "search", true, ()))
				.unwrap_or(true);

			if !auth {
				url.set_username("").map_err(|_| Error::new("Invalid URL", None))?;
			}
			if !fragment {
				url.set_fragment(None);
			}
			if !search {
				url.set_query(None);
			}

			Ok(url.to_string())
		}

		#[ion(get)]
		pub fn get_href(&self) -> String {
			self.url.to_string()
		}

		#[ion(set)]
		pub fn set_href(&mut self, input: String) -> Result<()> {
			match Url::parse(&input) {
				Ok(url) => {
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
		pub fn set_host(&mut self, host: Option<String>) -> Result<()> {
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

				self.url.set_port(port).map_err(|_| Error::new("Invalid URL", None))?;
			} else {
				self.url.set_host(None)?;
				self.url.set_port(None).map_err(|_| Error::new("Invalid URL", None))?;
			}
			Ok(())
		}

		#[ion(get)]
		pub fn get_hostname(&self) -> Option<String> {
			self.url.host_str().map(String::from)
		}

		#[ion(set)]
		pub fn set_hostname(&mut self, hostname: Option<String>) -> Result<()> {
			self.url
				.set_host(hostname.as_deref())
				.map_err(|error| Error::new(&error.to_string(), None))
		}

		#[ion(get)]
		pub fn get_port(&self) -> Option<u16> {
			self.url.port_or_known_default()
		}

		#[ion(set)]
		pub fn set_port(&mut self, #[ion(convert = EnforceRange)] port: Option<u16>) -> Result<()> {
			self.url.set_port(port).map_err(|_| Error::new("Invalid Port", None))
		}

		#[ion(get)]
		pub fn get_path(&self) -> String {
			String::from(self.url.path())
		}

		#[ion(set)]
		pub fn set_path(&mut self, path: String) -> Result<()> {
			self.url.set_path(&path);
			Ok(())
		}

		#[ion(get)]
		pub fn get_username(&self) -> String {
			String::from(self.url.username())
		}

		#[ion(set)]
		pub fn set_username(&mut self, username: String) -> Result<()> {
			self.url.set_username(&username).map_err(|_| Error::new("Invalid URL", None))
		}

		#[ion(get)]
		pub fn get_password(&self) -> Option<String> {
			self.url.password().map(String::from)
		}

		#[ion(set)]
		pub fn set_password(&mut self, password: Option<String>) -> Result<()> {
			self.url.set_password(password.as_deref()).map_err(|_| Error::new("Invalid URL", None))
		}

		#[ion(get)]
		pub fn get_search(&self) -> Option<String> {
			self.url.query().map(String::from)
		}

		#[ion(set)]
		pub fn set_search(&mut self, search: Option<String>) {
			self.url.set_query(search.as_deref());
		}

		#[ion(get)]
		pub fn get_hash(&self) -> Option<String> {
			self.url.fragment().map(String::from)
		}

		#[ion(set)]
		pub fn set_hash(&mut self, hash: Option<String>) {
			self.url.set_fragment(hash.as_deref());
		}
	}
}

#[js_fn]
fn domainToASCII(domain: String, strict: Option<bool>) -> Result<String> {
	let strict = strict.unwrap_or(false);
	let domain = if !strict {
		domain_to_ascii(&domain)
	} else {
		domain_to_ascii_strict(&domain)
	};
	domain.map_err(|e| e.into())
}

#[js_fn]
fn domainToUnicode(domain: String) -> String {
	domain_to_unicode(&domain).0
}

const FUNCTIONS: &[JSFunctionSpec] = &[function_spec!(domainToASCII, 0), function_spec!(domainToUnicode, 0), JSFunctionSpec::ZERO];

#[derive(Default)]
pub struct UrlM;

impl NativeModule for UrlM {
	const NAME: &'static str = "url";
	const SOURCE: &'static str = include_str!("url.js");

	fn module<'cx>(cx: &'cx Context) -> Option<Object<'cx>> {
		let mut url = Object::new(cx);
		if url.define_methods(cx, FUNCTIONS) {
			class::URL::init_class(cx, &mut url);
			return Some(url);
		}
		None
	}
}
