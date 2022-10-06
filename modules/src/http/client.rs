/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::time::Duration;

use hyper::client::HttpConnector;
use mozjs::conversions::{ConversionBehavior, ConversionResult, FromJSValConvertible};
use mozjs::rust::HandleValue;
use once_cell::sync::OnceCell;

pub use class::*;
use ion::Context;

pub(crate) static GLOBAL_CLIENT: OnceCell<hyper::Client<HttpConnector>> = OnceCell::new();

pub(crate) fn default_client() -> hyper::Client<HttpConnector> {
	let mut client = hyper::Client::builder();

	client.pool_idle_timeout(Duration::from_secs(60));
	client.pool_max_idle_per_host(usize::MAX);
	client.retry_canceled_requests(true);
	client.set_host(false);

	client.build_http()
}

#[derive(Derivative, FromJSVal)]
#[derivative(Default)]
pub struct ClientOptions {
	#[ion(default)]
	keep_alive: bool,
	#[ion(convert = ConversionBehavior::Clamp, default)]
	#[derivative(Default(value = "60"))]
	keep_alive_timeout: u64,
	#[ion(convert = ConversionBehavior::Clamp, default)]
	#[derivative(Default(value = "u64::MAX"))]
	max_idle_sockets: u64,
	#[ion(default = true)]
	#[derivative(Default(value = "true"))]
	retry_cancelled: bool,
}

#[derive(Clone)]
pub enum ClientRequestOptions {
	Global,
	New,
	Client(Client),
}

impl ClientRequestOptions {
	pub fn into_client(self) -> hyper::Client<HttpConnector> {
		use ClientRequestOptions as CRO;
		match self {
			CRO::Global => GLOBAL_CLIENT.get().unwrap().clone(),
			CRO::New => default_client(),
			CRO::Client(client) => client.client,
		}
	}
}

impl Default for ClientRequestOptions {
	fn default() -> ClientRequestOptions {
		ClientRequestOptions::Global
	}
}

impl FromJSValConvertible for ClientRequestOptions {
	type Config = ();

	unsafe fn from_jsval(cx: Context, val: HandleValue, _: ()) -> Result<ConversionResult<ClientRequestOptions>, ()> {
		if val.is_undefined() {
			Ok(ConversionResult::Success(ClientRequestOptions::Global))
		} else if val.is_boolean() {
			if val.to_boolean() {
				Ok(ConversionResult::Success(ClientRequestOptions::Global))
			} else {
				Ok(ConversionResult::Success(ClientRequestOptions::New))
			}
		} else {
			match Client::from_jsval(cx, val, ())? {
				ConversionResult::Success(client) => Ok(ConversionResult::Success(ClientRequestOptions::Client(client))),
				ConversionResult::Failure(f) => Ok(ConversionResult::Failure(f)),
			}
		}
	}
}

#[js_class]
mod class {
	use std::ops::Deref;
	use std::time::Duration;

	use hyper::client::HttpConnector;
	use mozjs::conversions::{ConversionResult, FromJSValConvertible};
	use mozjs::rust::HandleValue;

	use ion::class::class_from_jsval;
	use ion::Context;

	use crate::http::client::ClientOptions;

	#[derive(Clone)]
	pub struct Client {
		pub(crate) client: hyper::Client<HttpConnector>,
	}

	impl Client {
		#[ion(constructor)]
		pub fn constructor(options: Option<ClientOptions>) -> Client {
			let options = options.unwrap_or_default();
			let mut client = hyper::Client::builder();

			if options.keep_alive {
				client.pool_idle_timeout(Duration::from_millis(options.keep_alive_timeout));
				client.pool_max_idle_per_host(options.max_idle_sockets as usize);
			} else {
				client.pool_idle_timeout(None);
				client.pool_max_idle_per_host(0);
			}

			client.retry_canceled_requests(options.retry_cancelled);
			client.set_host(false);

			let client = client.build_http();
			Client { client }
		}
	}

	impl Deref for Client {
		type Target = hyper::Client<HttpConnector>;

		fn deref(&self) -> &hyper::Client<HttpConnector> {
			&self.client
		}
	}

	impl FromJSValConvertible for Client {
		type Config = ();

		unsafe fn from_jsval(cx: Context, val: HandleValue, _: ()) -> Result<ConversionResult<Client>, ()> {
			class_from_jsval(cx, val)
		}
	}
}
