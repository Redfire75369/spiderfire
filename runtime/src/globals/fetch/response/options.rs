/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use http::StatusCode;
use mozjs::conversions::ConversionBehavior;
use ion::{Context, Value, Result, Error, ErrorKind};
use ion::conversions::FromValue;
use crate::globals::fetch::header::HeadersInit;

#[derive(Default, FromValue)]
pub struct ResponseInit {
	#[ion(default)]
	pub(crate) headers: HeadersInit,

	#[ion(default, parser = |s| parse_status(cx, s))]
	pub(crate) status: StatusCode,
	#[ion(default)]
	pub(crate) status_text: String,
}

fn parse_status<'cx: 'v, 'v>(cx: &'cx Context, status: Value<'v>) -> Result<StatusCode> {
	let code = u16::from_value(cx, &status, true, ConversionBehavior::Clamp).map(StatusCode::from_u16);

	match code {
		Ok(Ok(code)) => Ok(code),
		_ => Err(Error::new("Invalid response status code", ErrorKind::Range)),
	}
}
