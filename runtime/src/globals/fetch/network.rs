/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::str::FromStr;

use bytes::Bytes;
use futures::future::{Either, select};
use http::{Method, StatusCode, Uri};
use http::header::{CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_LOCATION, CONTENT_TYPE, HOST, LOCATION};
use hyper::{Body, Client};
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use url::Url;

use ion::{Error, Exception, ResultExc};

use crate::globals::fetch::{Request, Response};
use crate::globals::fetch::request::{add_host_header, clone_request, RequestRedirect};

pub async fn request_internal(request: Request, client: Client<HttpsConnector<HttpConnector>>) -> ResultExc<Response> {
	let signal = request.signal.poll();
	let send = Box::pin(send_requests(request, client));
	match select(send, signal).await {
		Either::Left((response, _)) => response,
		Either::Right((exception, _)) => Err(Exception::Other(exception)),
	}
}

pub(crate) async fn send_requests(mut req: Request, client: Client<HttpsConnector<HttpConnector>>) -> ResultExc<Response> {
	let mut redirections = 0;

	let mut request = req.clone()?;
	*request.request.body_mut() = Body::from(request.body.clone());

	*req.request.body_mut() = Body::from(req.body);
	let mut response = client.request(req.request).await?;
	let mut locations = vec![request.url.clone()];

	while response.status().is_redirection() {
		if redirections >= 20 {
			return Err(Error::new("Too Many Redirects", None).into());
		}
		let status = response.status();
		if status != StatusCode::SEE_OTHER && !request.body.is_empty() {
			return Err(Error::new("Redirected with a Body", None).into());
		}

		match req.redirect {
			RequestRedirect::Follow => {
				let method = request.request.method().clone();

				if let Some(location) = response.headers().get(LOCATION) {
					let location = location.to_str()?;
					let url = {
						let options = Url::options();
						options.base_url(Some(&request.url));
						options.parse(location)
					}?;

					redirections += 1;

					if ((status == StatusCode::MOVED_PERMANENTLY || status == StatusCode::FOUND) && method == Method::POST)
						|| (status == StatusCode::SEE_OTHER && (method != Method::GET && method != Method::HEAD))
					{
						*request.request.method_mut() = Method::GET;

						request.body = Bytes::new();
						*request.request.body_mut() = Body::empty();

						let headers = request.request.headers_mut();
						headers.remove(CONTENT_ENCODING);
						headers.remove(CONTENT_LANGUAGE);
						headers.remove(CONTENT_LOCATION);
						headers.remove(CONTENT_TYPE);
					}

					request.request.headers_mut().remove(HOST);
					add_host_header(request.request.headers_mut(), &url, true)?;

					locations.push(url.clone());
					*request.request.uri_mut() = Uri::from_str(url.as_str())?;

					let request = { clone_request(&request.request) }?;
					response = client.request(request).await?;
				} else {
					return Ok(Response::new(response, redirections, locations));
				}
			}
			RequestRedirect::Error => return Err(Error::new("Received Redirection", None).into()),
			RequestRedirect::Manual => return Ok(Response::new(response, redirections, locations)),
		}
	}

	Ok(Response::new(response, redirections, locations))
}
