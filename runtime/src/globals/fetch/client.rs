/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::sync::OnceLock;
use std::time::Duration;

use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;

use crate::globals::fetch::body::Body;

pub type Client = legacy::Client<HttpsConnector<HttpConnector>, Body>;

pub static GLOBAL_CLIENT: OnceLock<Client> = OnceLock::new();

pub fn default_client() -> Client {
	let https = HttpsConnectorBuilder::new().with_webpki_roots().https_or_http().enable_http1().build();

	let mut client = legacy::Client::builder(TokioExecutor::default());

	client.pool_idle_timeout(Duration::from_secs(60));
	client.pool_max_idle_per_host(usize::MAX);
	client.retry_canceled_requests(true);
	client.set_host(false);

	client.build(https)
}
