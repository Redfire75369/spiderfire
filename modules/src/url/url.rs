/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use idna::{domain_to_ascii, domain_to_unicode};
use mozjs::jsapi::JSFunctionSpec;

use ion::{Context, Object, Result};
use ion::ClassInitialiser;
use runtime::modules::NativeModule;

#[js_class]
mod class {
	use std::cmp::Ordering;

	use mozjs::conversions::ConversionBehavior::EnforceRange;
	use url::Url;

	use ion::{Error, Object, Result};

	pub struct URL {
		url: Url,
	}

	impl URL {
		#[constructor]
		fn constructor(input: String, base: Option<String>) -> Result<URL> {
			let options = Url::options();
			let base = base.as_ref().and_then(|base| Url::parse(base).ok());
			options.base_url(base.as_ref());
			options.parse(&input).map_err(|error| Error::new(&error.to_string()))
		}

		fn origin(#[this] this: &URL) -> String {
			this.url.origin().ascii_serialization()
		}

		fn toString(#[this] this: &URL) -> String {
			this.url.to_string()
		}

		fn toJSON(#[this] this: &URL) -> String {
			this.url.to_string()
		}

		fn format(#[this] this: &URL, options: Option<Object>) -> Result<String> {
			let mut url = this.url.clone();

			let auth = options.and_then(|options| options.get_as::<bool>(cx, "auth", ())).unwrap_or(true);
			let fragment = options.and_then(|options| options.get_as::<bool>(cx, "fragment", ())).unwrap_or(true);
			let search = options.and_then(|options| options.get_as::<bool>(cx, "search", ())).unwrap_or(true);

			if !auth {
				url.set_username("").map_err(|_| Error::new("Invalid URL"))?;
			}
			if !fragment {
				url.set_fragment(None);
			}
			if !search {
				url.set_query(None);
			}

			Ok(url.to_string())
		}

		#[get]
		fn get_href(#[this] this: &URL) -> String {
			this.url.to_string()
		}

		#[set]
		fn set_href(#[this] this: &mut URL, input: String) -> Result<()> {
			match Url::parse(&input) {
				Ok(url) => {
					this.url = url;
					Ok(())
				}
				Err(error) => Err(Error::new(&error.to_string())),
			}
		}

		#[get]
		fn get_protocol(#[this] this: &URL) -> String {
			String::from(this.url.scheme())
		}

		#[set]
		fn set_protocol(#[this] this: &mut URL, protocol: String) -> Result<()> {
			this.url.set_scheme(&protocol).map_err(|_| Error::new("Invalid Protocol"))
		}

		#[get]
		fn get_host(#[this] this: &URL) -> Option<String> {
			this.url.host_str().map(|host| {
				if let Some(port) = this.url.port() {
					format!("{}:{}", host, port)
				} else {
					String::from(host)
				}
			})
		}

		#[set]
		fn set_host(#[this] this: &mut URL, host: Option<String>) -> Result<()> {
			if let Some(host) = host {
				let segments: Vec<&str> = host.split(':').collect();
				let (host, port) = match segments.len().cmp(&2) {
					Ordering::Less => Ok((segments[0], None)),
					Ordering::Greater => Err(Error::new("Invalid Host")),
					Ordering::Equal => {
						let port = match segments[1].parse::<u16>() {
							Ok(port) => Ok(port),
							Err(error) => Err(Error::new(&error.to_string())),
						}?;
						Ok((segments[0], Some(port)))
					}
				}?;

				this.url.set_host(Some(host))?;

				this.url.set_port(port).map_err(|_| Error::new("Invalid URL"))?;
			} else {
				this.url.set_host(None)?;
				this.url.set_port(None).map_err(|_| Error::new("Invalid URL"))?;
			}
			Ok(())
		}

		#[get]
		fn get_hostname(#[this] this: &URL) -> Option<String> {
			this.url.host_str().map(String::from)
		}

		#[set]
		fn set_hostname(#[this] this: &mut URL, hostname: Option<String>) -> Result<()> {
			this.url.set_host(hostname.as_deref()).map_err(|error| Error::new(&error.to_string()))
		}

		#[get]
		fn get_port(#[this] this: &URL) -> Option<u16> {
			this.url.port_or_known_default()
		}

		#[set]
		fn set_port(#[this] this: &mut URL, #[convert(EnforceRange)] port: Option<u16>) -> Result<()> {
			this.url.set_port(port).map_err(|_| Error::new("Invalid Port"))
		}

		#[get]
		fn get_path(#[this] this: &URL) -> String {
			String::from(this.url.path())
		}

		#[set]
		fn set_path(#[this] this: &mut URL, path: String) -> Result<()> {
			this.url.set_path(&path);
			Ok(())
		}

		#[get]
		fn get_username(#[this] this: &URL) -> String {
			String::from(this.url.username())
		}

		#[set]
		fn set_username(#[this] this: &mut URL, username: String) -> Result<()> {
			this.url.set_username(&username).map_err(|_| Error::new("Invalid URL"))
		}

		#[get]
		fn get_password(#[this] this: &URL) -> Option<String> {
			this.url.password().map(String::from)
		}

		#[set]
		fn set_password(#[this] this: &mut URL, password: Option<String>) -> Result<()> {
			this.url.set_password(password.as_deref()).map_err(|_| Error::new("Invalid URL"))
		}

		#[get]
		fn get_search(#[this] this: &URL) -> Option<String> {
			this.url.query().map(String::from)
		}

		#[set]
		fn set_search(#[this] this: &mut URL, search: Option<String>) {
			this.url.set_query(search.as_deref());
		}

		#[get]
		fn get_hash(#[this] this: &URL) -> Option<String> {
			this.url.fragment().map(String::from)
		}

		#[set]
		fn set_hash(#[this] this: &mut URL, hash: Option<String>) {
			this.url.set_fragment(hash.as_deref());
		}
	}
}

#[js_fn]
fn domainToASCII(domain: String) -> Result<String> {
	domain_to_ascii(&domain).map_err(|error| error.into())
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

	fn module(cx: Context) -> Option<Object> {
		let mut url = Object::new(cx);
		if url.define_methods(cx, FUNCTIONS) {
			class::URL::init_class(cx, &url);
			return Some(url);
		}
		None
	}
}
