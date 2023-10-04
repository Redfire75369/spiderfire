/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;

pub use class::*;
use ion::{Context, Value};
use ion::conversions::ConversionBehavior;
use ion::conversions::FromValue;
use runtime::globals::fetch::{default_client, GLOBAL_CLIENT};

#[derive(Derivative, FromValue)]
#[derivative(Default)]
pub struct ClientInit {
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

#[derive(Clone, Default)]
pub enum ClientRequestOptions {
	#[default]
	Global,
	New,
	Client(Client),
}

impl ClientRequestOptions {
	pub fn to_client(&self) -> hyper::Client<HttpsConnector<HttpConnector>> {
		use ClientRequestOptions as CRO;
		match self {
			CRO::Global => GLOBAL_CLIENT.get().unwrap().clone(),
			CRO::New => default_client(),
			CRO::Client(client) => client.client.clone(),
		}
	}
}

impl<'cx> FromValue<'cx> for ClientRequestOptions {
	type Config = ();
	fn from_value<'v>(cx: &'cx Context, value: &Value<'v>, _: bool, _: ()) -> ion::Result<ClientRequestOptions>
	where
		'cx: 'v,
	{
		if value.handle().is_undefined() {
			Ok(ClientRequestOptions::Global)
		} else if value.handle().is_boolean() {
			if value.handle().to_boolean() {
				Ok(ClientRequestOptions::Global)
			} else {
				Ok(ClientRequestOptions::New)
			}
		} else {
			let client = Client::from_value(cx, value, true, ())?;
			Ok(ClientRequestOptions::Client(client))
		}
	}
}

#[js_class]
mod class {
	use std::ops::Deref;
	use std::time::Duration;

	use hyper::client::HttpConnector;
	use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};

	use crate::http::client::ClientInit;

	#[derive(Clone)]
	#[ion(from_value, into_value)]
	pub struct Client {
		pub(crate) client: hyper::Client<HttpsConnector<HttpConnector>>,
	}

	impl Client {
		#[ion(constructor)]
		pub fn constructor(options: Option<ClientInit>) -> Client {
			let options = options.unwrap_or_default();

			let https = HttpsConnectorBuilder::new().with_webpki_roots().https_or_http().enable_http1().build();

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

			let client = client.build(https);
			Client { client }
		}
	}

	impl Deref for Client {
		type Target = hyper::Client<HttpsConnector<HttpConnector>>;

		fn deref(&self) -> &hyper::Client<HttpsConnector<HttpConnector>> {
			&self.client
		}
	}
}
