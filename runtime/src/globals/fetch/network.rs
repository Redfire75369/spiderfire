/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::mem::take;
use std::str::FromStr;

use futures::future::{Either, select};
use http::{Method, StatusCode, Uri};
use http::header::{CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_LOCATION, CONTENT_TYPE, HOST, LOCATION};
use hyper::{Body, Client};
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use mozjs::jsapi::JSObject;
use url::Url;

use ion::{ClassDefinition, Context, Error, Exception, Local, Object, ResultExc};
use ion::class::Reflector;

use crate::globals::abort::AbortSignal;
use crate::globals::fetch::{Headers, Request, Response};
use crate::globals::fetch::body::FetchBody;
use crate::globals::fetch::header::HeadersKind;
use crate::globals::fetch::request::{clone_request, RequestRedirect};

pub async fn request_internal<'c, 'o>(
	cx: &Context<'c>, request: &Object<'o>, client: Client<HttpsConnector<HttpConnector>>,
) -> ResultExc<*mut JSObject> {
	let request = Request::get_private(request);
	let signal = Object::from(unsafe { Local::from_heap(&request.signal_object) });
	let signal = AbortSignal::get_private(&signal).signal.clone().poll();
	let send = Box::pin(send_requests(cx, request, client));
	match select(send, signal).await {
		Either::Left((response, _)) => response,
		Either::Right((exception, _)) => Err(Exception::Other(exception)),
	}
}

pub(crate) async fn send_requests<'c>(cx: &Context<'c>, req: &Request, client: Client<HttpsConnector<HttpConnector>>) -> ResultExc<*mut JSObject> {
	let mut redirections = 0;

	let mut request = req.clone()?;
	*request.request.body_mut() = request.body.to_http_body();

	let headers = Object::from(unsafe { Local::from_heap(&req.headers) });
	let headers = Headers::get_private(&headers);
	*request.request.headers_mut() = headers.headers.clone();

	let mut response = client.request(clone_request(&req.request)?).await?;
	let mut url = request.url.clone();

	while response.status().is_redirection() {
		if redirections >= 20 {
			return Err(Error::new("Too Many Redirects", None).into());
		}
		let status = response.status();
		if status != StatusCode::SEE_OTHER && !request.body.is_empty() {
			return Err(Error::new("Redirected with a Body", None).into());
		}

		match request.redirect {
			RequestRedirect::Follow => {
				let method = request.request.method().clone();

				if let Some(location) = response.headers().get(LOCATION) {
					let location = location.to_str()?;
					url = {
						let options = Url::options();
						options.base_url(Some(&request.url));
						options.parse(location)
					}?;

					redirections += 1;

					if ((status == StatusCode::MOVED_PERMANENTLY || status == StatusCode::FOUND) && method == Method::POST)
						|| (status == StatusCode::SEE_OTHER && (method != Method::GET && method != Method::HEAD))
					{
						*request.request.method_mut() = Method::GET;

						request.body = FetchBody::default();
						*request.request.body_mut() = Body::empty();

						let headers = request.request.headers_mut();
						headers.remove(CONTENT_ENCODING);
						headers.remove(CONTENT_LANGUAGE);
						headers.remove(CONTENT_LOCATION);
						headers.remove(CONTENT_TYPE);
					}

					request.request.headers_mut().remove(HOST);

					*request.request.uri_mut() = Uri::from_str(url.as_str())?;

					let request = { clone_request(&request.request) }?;
					response = client.request(request).await?;
				} else {
					break;
				}
			}
			RequestRedirect::Error => return Err(Error::new("Received Redirection", None).into()),
			RequestRedirect::Manual => break,
		}
	}

	let mut response = Response::new(response, url, redirections > 0);

	let headers = Headers {
		reflector: Reflector::default(),
		headers: take(response.response.headers_mut()),
		kind: HeadersKind::Response,
	};
	response.headers.set(Headers::new_object(cx, Box::new(headers)));

	let response = Object::from(cx.root_object(Response::new_object(cx, Box::new(response))));
	Ok(response.handle().get())
}
