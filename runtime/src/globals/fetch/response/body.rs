/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use hyper::{Body, body};
use ion::Result;
use crate::globals::fetch::body::FetchBody;

#[derive(Traceable)]
pub enum ResponseBody {
	Fetch(FetchBody),
	Hyper(#[trace(no_trace)] Body),
}

impl ResponseBody {
	pub async fn read_to_bytes(self) -> Result<Vec<u8>> {
		let body = match self {
			ResponseBody::Fetch(body) => body.to_http_body(),
			ResponseBody::Hyper(body) => body,
		};

		match body::to_bytes(body).await {
			Ok(bytes) => Ok(bytes.to_vec()),
			Err(error) => Err(error.into()),
		}
	}
}
