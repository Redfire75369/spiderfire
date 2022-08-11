/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use syn::TypePath;

pub(crate) fn type_ends_with(ty: &TypePath, ident: &str) -> bool {
	if let Some(last) = ty.path.segments.last() {
		last.ident == ident
	} else {
		false
	}
}
