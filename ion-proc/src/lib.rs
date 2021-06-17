/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn js_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = proc_macro2::TokenStream::from(item);
	let new = quote! {
        js_fn_m!(#item)
    };
    TokenStream::from(new)
}
