/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::ptr;

use idna::{domain_to_ascii, domain_to_unicode};
use mozjs::conversions::{jsstr_to_string, ToJSValConvertible};
use mozjs::conversions::ConversionBehavior::EnforceRange;
use mozjs::glue::JS_GetReservedSlot;
use mozjs::jsapi::{
	HandleObject, JS_DefineFunctions, JS_InitClass, JS_NewObjectForConstructor, JS_SetReservedSlot, JSClass, JSFunctionSpec, JSPropertySpec,
};
use mozjs::jsval::{JSVal, NullValue};
use url::Url;

use ion::{Arguments, class_reserved_slots, Context, Error, Object, Result};
use ion::flags::PropertyFlags;
use runtime::modules::NativeModule;

unsafe fn get_url(cx: Context, this: Object) -> Url {
	let mut value = NullValue();
	JS_GetReservedSlot(*this, 0, &mut value);
	Url::parse(&jsstr_to_string(cx, value.to_string())).unwrap()
}

unsafe fn set_url(cx: Context, this: Object, url: Url) {
	rooted!(in(cx) let mut string = NullValue());
	url.to_string().to_jsval(cx, string.handle_mut());
	JS_SetReservedSlot(*this, 0, &string.get());
}

#[js_fn]
unsafe fn constructor(cx: Context, args: &Arguments, input: String, base: Option<String>) -> Result<Object> {
	if !args.is_constructing() {
		return Err(Error::Error(String::from("This constructor must be called with \"new\".")));
	}
	let options = Url::options();
	let base = base.as_ref().map(|base| Url::parse(base).ok()).flatten();
	options.base_url(base.as_ref());
	match options.parse(&input) {
		Ok(url) => {
			rooted!(in(cx) let this = JS_NewObjectForConstructor(cx, &URL_CLASS, &args.call_args()));
			set_url(cx, Object::from(this.get()), url);
			Ok(Object::from(this.get()))
		}
		Err(error) => Err(Error::Error(error.to_string())),
	}
}

#[js_fn]
unsafe fn href(#[this] this: Object) -> Result<JSVal> {
	let mut value = NullValue();
	JS_GetReservedSlot(*this, 0, &mut value);
	Ok(value)
}

#[js_fn]
unsafe fn protocol(cx: Context, #[this] this: Object) -> Result<String> {
	let url = get_url(cx, this);
	Ok(format!("{}:", url.scheme()))
}

#[js_fn]
unsafe fn host(cx: Context, #[this] this: Object) -> Result<Option<String>> {
	let url = get_url(cx, this);
	let host = url.host_str().map(|host| {
		if let Some(port) = url.port() {
			format!("{}:{}", host, port)
		} else {
			String::from(host)
		}
	});
	Ok(host)
}

#[js_fn]
unsafe fn hostname(cx: Context, #[this] this: Object) -> Result<Option<String>> {
	let url = get_url(cx, this);
	Ok(url.host_str().map(String::from))
}

#[js_fn]
unsafe fn port(cx: Context, #[this] this: Object) -> Result<Option<u16>> {
	let url = get_url(cx, this);
	Ok(url.port_or_known_default())
}

#[js_fn]
unsafe fn path(cx: Context, #[this] this: Object) -> Result<String> {
	let url = get_url(cx, this);
	Ok(String::from(url.path()))
}

#[js_fn]
unsafe fn username(cx: Context, #[this] this: Object) -> Result<String> {
	let url = get_url(cx, this);
	Ok(String::from(url.username()))
}

#[js_fn]
unsafe fn password(cx: Context, #[this] this: Object) -> Result<Option<String>> {
	let url = get_url(cx, this);
	Ok(url.password().map(String::from))
}

#[js_fn]
unsafe fn search(cx: Context, #[this] this: Object) -> Result<Option<String>> {
	let url = get_url(cx, this);
	Ok(url.query().map(String::from))
}

#[js_fn]
unsafe fn hash(cx: Context, #[this] this: Object) -> Result<Option<String>> {
	let url = get_url(cx, this);
	Ok(url.fragment().map(String::from))
}

#[js_fn]
unsafe fn set_href(cx: Context, #[this] this: Object, input: String) -> Result<()> {
	match Url::parse(&input) {
		Ok(url) => {
			set_url(cx, this, url);
			Ok(())
		}
		Err(error) => Err(Error::Error(error.to_string())),
	}
}

#[js_fn]
unsafe fn set_protocol(cx: Context, #[this] this: Object, protocol: String) -> Result<()> {
	let mut url = get_url(cx, this);
	let result = url.set_scheme(&protocol).map_err(|_| Error::Error(String::from("Invalid Protocol")));
	set_url(cx, this, url);
	result
}

#[js_fn]
unsafe fn set_host(cx: Context, #[this] this: Object, host: Option<String>) -> Result<()> {
	let mut url = get_url(cx, this);

	if let Some(host) = host {
		let segments: Vec<&str> = host.split(':').collect();
		let (host, port) = match segments.len().cmp(&2) {
			Ordering::Less => (segments[0], None),
			Ordering::Greater => return Err(Error::Error(String::from("Invalid Host"))),
			Ordering::Equal => {
				let port = match segments[1].parse::<u16>() {
					Ok(port) => port,
					Err(error) => return Err(Error::Error(error.to_string())),
				};
				(segments[0], Some(port))
			}
		};

		if let Err(error) = url.set_host(Some(host)) {
			return Err(Error::Error(error.to_string()));
		}

		let _ = url.set_port(port);
	} else {
		let _ = url.set_host(None);
		let _ = url.set_port(None);
	}

	set_url(cx, this, url);
	Ok(())
}

#[js_fn]
unsafe fn set_hostname(cx: Context, #[this] this: Object, hostname: Option<String>) -> Result<()> {
	let mut url = get_url(cx, this);
	let result = url.set_host(hostname.as_deref()).map_err(|error| Error::Error(error.to_string()));
	set_url(cx, this, url);
	result
}

#[js_fn]
unsafe fn set_port(cx: Context, #[this] this: Object, #[convert(EnforceRange)] port: Option<u16>) -> Result<()> {
	let mut url = get_url(cx, this);
	let _ = url.set_port(port);
	set_url(cx, this, url);
	Ok(())
}

#[js_fn]
unsafe fn set_path(cx: Context, #[this] this: Object, input: String) -> Result<()> {
	let mut url = get_url(cx, this);
	url.set_path(&input);
	set_url(cx, this, url);
	Ok(())
}

#[js_fn]
unsafe fn set_username(cx: Context, #[this] this: Object, input: String) -> Result<()> {
	let mut url = get_url(cx, this);
	let _ = url.set_username(&input);
	set_url(cx, this, url);
	Ok(())
}

#[js_fn]
unsafe fn set_password(cx: Context, #[this] this: Object, input: Option<String>) -> Result<()> {
	let mut url = get_url(cx, this);
	let _ = url.set_password(input.as_deref());
	set_url(cx, this, url);
	Ok(())
}

#[js_fn]
unsafe fn set_search(cx: Context, #[this] this: Object, input: Option<String>) -> Result<()> {
	let mut url = get_url(cx, this);
	url.set_query(input.as_deref());
	set_url(cx, this, url);
	Ok(())
}

#[js_fn]
unsafe fn set_hash(cx: Context, #[this] this: Object, input: Option<String>) -> Result<()> {
	let mut url = get_url(cx, this);
	url.set_fragment(input.as_deref());
	set_url(cx, this, url);
	Ok(())
}

#[js_fn]
unsafe fn origin(cx: Context, #[this] this: Object) -> Result<String> {
	let url = get_url(cx, this);
	Ok(url.origin().ascii_serialization())
}

#[js_fn]
unsafe fn toString(#[this] this: Object) -> Result<JSVal> {
	let mut value = NullValue();
	JS_GetReservedSlot(*this, 0, &mut value);
	Ok(value)
}

#[js_fn]
unsafe fn toJSON(#[this] this: Object) -> Result<JSVal> {
	let mut value = NullValue();
	JS_GetReservedSlot(*this, 0, &mut value);
	Ok(value)
}

#[js_fn]
unsafe fn format(cx: Context, #[this] this: Object, options: Option<Object>) -> Result<String> {
	let mut url = get_url(cx, this);

	let auth = options.map(|options| options.get_as::<bool>(cx, "auth", ())).flatten().unwrap_or(true);
	let fragment = options
		.map(|options| options.get_as::<bool>(cx, "fragment", ()))
		.flatten()
		.unwrap_or(true);
	let search = options.map(|options| options.get_as::<bool>(cx, "search", ())).flatten().unwrap_or(true);

	if !auth {
		let _ = url.set_username("");
	}
	if !fragment {
		url.set_fragment(None);
	}
	if !search {
		url.set_query(None);
	}

	Ok(url.to_string())
}

#[js_fn]
fn domainToASCII(domain: String) -> Result<String> {
	domain_to_ascii(&domain).map_err(|error| Error::Error(error.to_string()))
}

#[js_fn]
fn domainToUnicode(domain: String) -> Result<String> {
	Ok(domain_to_unicode(&domain).0)
}

static PROPERTIES: &[JSPropertySpec] = &[
	property_spec_getter_setter!(href, set_href),
	property_spec_getter_setter!(protocol, set_protocol),
	property_spec_getter_setter!(host, set_host),
	property_spec_getter_setter!(hostname, set_hostname),
	property_spec_getter_setter!(port, set_port),
	property_spec_getter_setter!(path, set_path),
	property_spec_getter_setter!(username, set_username),
	property_spec_getter_setter!(password, set_password),
	property_spec_getter_setter!(search, set_search),
	property_spec_getter_setter!(hash, set_hash),
	property_spec_getter!(origin),
	JSPropertySpec::ZERO,
];

static METHODS: &[JSFunctionSpec] = &[
	function_spec!(toString, "toString", 0, PropertyFlags::CONSTANT),
	function_spec!(toJSON, "toJSON", 0, PropertyFlags::CONSTANT),
	function_spec!(format, "format", 0, PropertyFlags::CONSTANT),
	JSFunctionSpec::ZERO,
];

static URL_CLASS: JSClass = JSClass {
	name: "URL\0".as_ptr() as *const i8,
	flags: class_reserved_slots(1),
	cOps: ptr::null_mut(),
	spec: ptr::null_mut(),
	ext: ptr::null_mut(),
	oOps: ptr::null_mut(),
};

const FUNCTIONS: &[JSFunctionSpec] = &[function_spec!(domainToASCII, 0), function_spec!(domainToUnicode, 0), JSFunctionSpec::ZERO];

#[derive(Default)]
pub struct UrlM;

impl NativeModule for UrlM {
	const NAME: &'static str = "url";
	const SOURCE: &'static str = include_str!("url.js");

	fn module(cx: Context) -> Option<Object> {
		rooted!(in(cx) let url = *Object::new(cx));
		if unsafe { JS_DefineFunctions(cx, url.handle().into(), FUNCTIONS.as_ptr()) } {
			let class = unsafe {
				JS_InitClass(
					cx,
					url.handle().into(),
					HandleObject::null(),
					&URL_CLASS,
					Some(constructor),
					1,
					PROPERTIES.as_ptr() as *const _,
					METHODS.as_ptr() as *const _,
					ptr::null_mut(),
					ptr::null_mut(),
				)
			};
			if !class.is_null() {
				return Some(Object::from(url.get()));
			}
		}
		None
	}
}
