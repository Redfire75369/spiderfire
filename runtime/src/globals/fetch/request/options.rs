/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use url::Url;

use ion::{Context, Error, ErrorKind, Result, Value};
use ion::conversions::{FromValue, ToValue};

use crate::globals::abort::AbortSignal;
use crate::globals::fetch::body::FetchBody;
use crate::globals::fetch::header::HeadersInit;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Default)]
pub enum RequestDestination {
	#[default]
	None,
	Audio,
	AudioWorklet,
	Document,
	Embed,
	Frame,
	Font,
	IFrame,
	Image,
	Manifest,
	Object,
	PaintWorklet,
	Report,
	Script,
	SharedWorker,
	Style,
	Track,
	Video,
	Worker,
	Xslt,
}

impl Display for RequestDestination {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		use RequestDestination as D;
		f.write_str(match self {
			D::None => "",
			D::Audio => "audio",
			D::AudioWorklet => "audioworklet",
			D::Document => "document",
			D::Embed => "embed",
			D::Frame => "frame",
			D::Font => "font",
			D::IFrame => "iframe",
			D::Image => "image",
			D::Manifest => "manifest",
			D::Object => "object",
			D::PaintWorklet => "paintworklet",
			D::Report => "report",
			D::Script => "script",
			D::SharedWorker => "sharedworker",
			D::Style => "style",
			D::Track => "track",
			D::Video => "video",
			D::Worker => "worker",
			D::Xslt => "xslt",
		})
	}
}

impl<'cx> ToValue<'cx> for RequestDestination {
	fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.to_string().to_value(cx, value);
	}
}

#[derive(Clone, Default, Debug)]
pub enum Referrer {
	#[allow(clippy::enum_variant_names)]
	NoReferrer,
	#[default]
	Client,
	Url(Url),
}

impl FromStr for Referrer {
	type Err = Error;

	fn from_str(referrer: &str) -> Result<Referrer> {
		if referrer.is_empty() {
			Ok(Referrer::NoReferrer)
		} else {
			let url = Url::parse(referrer).map_err(|e| Error::new(&e.to_string(), ErrorKind::Type))?;

			if url.scheme() == "about" && url.path() == "client" {
				Ok(Referrer::Client)
			} else {
				Ok(Referrer::Url(url))
			}
		}
	}
}

impl<'cx> FromValue<'cx> for Referrer {
	type Config = ();

	fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, strict: bool, _: ()) -> Result<Referrer>
	where
		'cx: 'v,
	{
		let referrer = String::from_value(cx, value, strict, ())?;
		Referrer::from_str(&referrer)
	}
}

#[derive(Copy, Clone, Default, Debug)]
pub enum ReferrerPolicy {
	#[default]
	None,
	NoReferrer,
	NoReferrerWhenDowngrade,
	Origin,
	OriginWhenCrossOrigin,
	SameOrigin,
	StrictOrigin,
	StrictOriginWhenCrossOrigin,
	UnsafeUrl,
}

impl FromStr for ReferrerPolicy {
	type Err = Error;

	fn from_str(policy: &str) -> Result<ReferrerPolicy> {
		use ReferrerPolicy as RP;
		match policy {
			"" => Ok(RP::None),
			"no-referrer" => Ok(RP::NoReferrer),
			"no-referrer-when-downgrade" => Ok(RP::NoReferrerWhenDowngrade),
			"origin" => Ok(RP::Origin),
			"origin-when-cross-origin" => Ok(RP::OriginWhenCrossOrigin),
			"same-origin" => Ok(RP::SameOrigin),
			"strict-origin" => Ok(RP::StrictOrigin),
			"strict-origin-when-cross-origin" => Ok(RP::StrictOriginWhenCrossOrigin),
			"unsafe-url" => Ok(RP::UnsafeUrl),
			_ => Err(Error::new("Invalid value for Enumeration ReferrerPolicy", ErrorKind::Type)),
		}
	}
}

impl<'cx> FromValue<'cx> for ReferrerPolicy {
	type Config = ();

	fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<ReferrerPolicy>
	where
		'cx: 'v,
	{
		let policy = String::from_value(cx, value, true, ())?;
		ReferrerPolicy::from_str(&policy)
	}
}

#[derive(Copy, Clone, Debug, Default)]
pub enum RequestMode {
	SameOrigin,
	Cors,
	#[default]
	NoCors,
	Navigate,
	#[allow(dead_code)]
	Websocket,
}

impl FromStr for RequestMode {
	type Err = Error;

	fn from_str(mode: &str) -> Result<RequestMode> {
		use RequestMode as RM;
		match mode {
			"same-origin" => Ok(RM::SameOrigin),
			"cors" => Ok(RM::Cors),
			"no-cors" => Ok(RM::NoCors),
			"navigate" => Ok(RM::Navigate),
			_ => Err(Error::new("Invalid value for Enumeration RequestMode", ErrorKind::Type)),
		}
	}
}

impl<'cx> FromValue<'cx> for RequestMode {
	type Config = ();

	fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<RequestMode>
	where
		'cx: 'v,
	{
		let mode = String::from_value(cx, value, true, ())?;
		RequestMode::from_str(&mode)
	}
}

#[derive(Copy, Clone, Debug, Default)]
pub enum RequestCredentials {
	Omit,
	#[default]
	SameOrigin,
	Include,
}

impl FromStr for RequestCredentials {
	type Err = Error;

	fn from_str(credentials: &str) -> Result<RequestCredentials> {
		use RequestCredentials as RC;
		match credentials {
			"omit" => Ok(RC::Omit),
			"same-origin" => Ok(RC::SameOrigin),
			"include" => Ok(RC::Include),
			_ => Err(Error::new("Invalid value for Enumeration RequestCredentials", ErrorKind::Type)),
		}
	}
}

impl<'cx> FromValue<'cx> for RequestCredentials {
	type Config = ();

	fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<RequestCredentials>
	where
		'cx: 'v,
	{
		let mode = String::from_value(cx, value, true, ())?;
		RequestCredentials::from_str(&mode)
	}
}

#[derive(Copy, Clone, Debug, Default)]
pub enum RequestCache {
	#[default]
	Default,
	NoStore,
	Reload,
	NoCache,
	ForceCache,
	OnlyIfCached,
}

impl FromStr for RequestCache {
	type Err = Error;

	fn from_str(credentials: &str) -> Result<RequestCache> {
		use RequestCache as RC;
		match credentials {
			"default" => Ok(RC::Default),
			"no-store" => Ok(RC::NoStore),
			"reload" => Ok(RC::Reload),
			"no-cache" => Ok(RC::NoCache),
			"force-cache" => Ok(RC::ForceCache),
			"only-if-cached" => Ok(RC::OnlyIfCached),
			_ => Err(Error::new("Invalid value for Enumeration RequestCache", ErrorKind::Type)),
		}
	}
}

impl<'cx> FromValue<'cx> for RequestCache {
	type Config = ();

	fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<RequestCache>
	where
		'cx: 'v,
	{
		let mode = String::from_value(cx, value, true, ())?;
		RequestCache::from_str(&mode)
	}
}

#[derive(Copy, Clone, Debug, Default)]
pub enum RequestRedirect {
	#[default]
	Follow,
	Error,
	Manual,
}

impl FromStr for RequestRedirect {
	type Err = Error;

	fn from_str(redirect: &str) -> Result<RequestRedirect> {
		use RequestRedirect as RR;
		match redirect {
			"follow" => Ok(RR::Follow),
			"error" => Ok(RR::Error),
			"manual" => Ok(RR::Manual),
			_ => Err(Error::new("Invalid value for Enumeration RequestRedirect", ErrorKind::Type)),
		}
	}
}

impl<'cx> FromValue<'cx> for RequestRedirect {
	type Config = ();

	fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> Result<RequestRedirect>
	where
		'cx: 'v,
	{
		let redirect = String::from_value(cx, value, true, ())?;
		RequestRedirect::from_str(&redirect)
	}
}

#[derive(Derivative, FromValue)]
#[derivative(Default)]
pub struct RequestInit {
	#[ion(default)]
	pub(crate) headers: HeadersInit,
	#[ion(default)]
	pub(crate) body: Option<FetchBody>,

	#[allow(dead_code)]
	#[ion(default)]
	pub(crate) referrer: Referrer,
	#[allow(dead_code)]
	#[ion(default)]
	pub(crate) referrer_policy: ReferrerPolicy,

	#[allow(dead_code)]
	pub(crate) mode: Option<RequestMode>,
	#[allow(dead_code)]
	#[ion(default)]
	pub(crate) credentials: RequestCredentials,
	#[allow(dead_code)]
	#[ion(default)]
	pub(crate) cache: RequestCache,
	#[ion(default)]
	pub(crate) redirect: RequestRedirect,

	#[allow(dead_code)]
	pub(crate) integrity: Option<String>,

	#[allow(dead_code)]
	#[derivative(Default(value = "true"))]
	#[ion(default = true)]
	pub(crate) keepalive: bool,
	#[allow(dead_code)]
	#[derivative(Default(value = "true"))]
	#[ion(default = true)]
	pub(crate) is_reload_navigation: bool,
	#[allow(dead_code)]
	#[derivative(Default(value = "true"))]
	#[ion(default = true)]
	pub(crate) is_history_navigation: bool,

	#[ion(default)]
	pub(crate) signal: AbortSignal,

	pub(crate) auth: Option<String>,
	#[derivative(Default(value = "true"))]
	#[ion(default = true)]
	pub(crate) set_host: bool,
}

#[derive(Default, FromValue)]
pub struct RequestBuilderInit {
	pub(crate) method: Option<String>,
	#[ion(default, inherit)]
	pub(crate) init: RequestInit,
}

impl RequestBuilderInit {
	pub fn from_request_init<O: Into<Option<RequestInit>>, M: Into<Option<String>>>(init: O, method: M) -> RequestBuilderInit {
		let init = init.into().unwrap_or_default();
		let method = method.into();
		RequestBuilderInit { method, init }
	}
}
