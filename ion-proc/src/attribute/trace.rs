/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::parse::{Parse, ParseStream, Result};

mod keywords {
	custom_keyword!(no_trace);
}

#[allow(dead_code)]
pub struct NoTraceFieldAttribute {
	kw: keywords::no_trace,
}

impl Parse for NoTraceFieldAttribute {
	fn parse(input: ParseStream) -> Result<Self> {
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::no_trace) {
			Ok(NoTraceFieldAttribute { kw: input.parse()? })
		} else {
			Err(lookahead.error())
		}
	}
}
