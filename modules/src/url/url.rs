/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use idna::{domain_to_ascii, domain_to_ascii_strict, domain_to_unicode};
use mozjs::jsapi::JSFunctionSpec;

use ion::{ClassDefinition, Context, Object, Result};
use ion::function::Opt;
use runtime::globals::url::{URL, URLSearchParams};
use runtime::module::NativeModule;

#[js_fn]
fn domainToASCII(domain: String, Opt(strict): Opt<bool>) -> Result<String> {
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

const FUNCTIONS: &[JSFunctionSpec] = &[
	function_spec!(domainToASCII, 0),
	function_spec!(domainToUnicode, 0),
	JSFunctionSpec::ZERO,
];

#[derive(Default)]
pub struct UrlM;

impl NativeModule for UrlM {
	const NAME: &'static str = "url";
	const SOURCE: &'static str = include_str!("url.js");

	fn module(cx: &Context) -> Option<Object> {
		let url = Object::new(cx);
		let global = Object::global(cx);

		if unsafe { url.define_methods(cx, FUNCTIONS) } {
			if let Some(global_url) = global.get(cx, "URL") {
				url.set(cx, "URL", &global_url);
			} else {
				URL::init_class(cx, &url);
			}

			if let Some(url_search_params) = global.get(cx, "URLSearchParams") {
				url.set(cx, "URLSearchParams", &url_search_params);
			} else {
				URLSearchParams::init_class(cx, &url);
			}

			return Some(url);
		}
		None
	}
}
