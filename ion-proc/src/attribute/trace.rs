/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::Error;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;

use crate::attribute::AttributeExt;

mod keywords {
	custom_keyword!(no_trace);
}

pub enum TraceAttributeArgument {
	NoTrace(keywords::no_trace),
}

impl Parse for TraceAttributeArgument {
	fn parse(input: ParseStream) -> Result<Self> {
		use TraceAttributeArgument as TAA;
		let lookahead = input.lookahead1();
		if lookahead.peek(keywords::no_trace) {
			Ok(TAA::NoTrace(input.parse()?))
		} else {
			Err(lookahead.error())
		}
	}
}

#[derive(Default)]
pub struct TraceAttribute {
	pub(crate) no_trace: bool,
}

impl Parse for TraceAttribute {
	fn parse(input: ParseStream) -> Result<TraceAttribute> {
		use TraceAttributeArgument as TAA;
		let mut attribute = TraceAttribute::default();
		let span = input.span();

		let args = Punctuated::<TraceAttributeArgument, Token![,]>::parse_terminated(input)?;
		for arg in args {
			match arg {
				TAA::NoTrace(_) => {
					if attribute.no_trace {
						return Err(Error::new(span, "Field cannot have multiple `no_trace` attributes."));
					}
					attribute.no_trace = true;
				}
			}
		}

		Ok(attribute)
	}
}

impl AttributeExt for TraceAttribute {}
