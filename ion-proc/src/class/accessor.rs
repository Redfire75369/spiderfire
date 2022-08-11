/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use proc_macro2::Ident;
use syn::{Error, ItemFn, Result};

use crate::class::Accessor;
use crate::class::method::impl_method;

pub(crate) fn get_accessor_name(ident: &Ident, is_setter: bool) -> String {
	let mut name = ident.to_string();
	let pat = if is_setter { "set_" } else { "get_" };
	if name.starts_with(pat) {
		name.drain(0..4);
	}
	name
}

pub(crate) fn impl_accessor(method: &ItemFn, is_setter: bool) -> Result<(ItemFn, bool)> {
	let expected_args = if is_setter { 1 } else { 0 };
	let error_message = if is_setter {
		format!("Expected Setter to have {} argument", expected_args)
	} else {
		format!("Expected Getter to have {} arguments", expected_args)
	};
	let error = Error::new_spanned(&method.sig, error_message);
	impl_method(method.clone(), |nargs| (nargs == expected_args).then(|| ()).ok_or(error)).map(|(method, _, this)| (method, this.is_some()))
}

pub(crate) fn insert_accessor(accessors: &mut HashMap<String, Accessor>, name: String, getter: Option<ItemFn>, setter: Option<ItemFn>) {
	match accessors.entry(name) {
		Entry::Occupied(mut o) => match (getter, setter) {
			(Some(g), Some(s)) => *o.get_mut() = (Some(g), Some(s)),
			(Some(g), None) => o.get_mut().0 = Some(g),
			(None, Some(s)) => o.get_mut().1 = Some(s),
			(None, None) => {}
		},
		Entry::Vacant(v) => {
			v.insert((getter, setter));
		},
	}
}

pub(crate) fn flatten_accessors(accessors: HashMap<String, Accessor>) -> Vec<ItemFn> {
	accessors
		.into_iter()
		.flat_map(|(_, (getter, setter))| [getter, setter])
		.filter_map(|p| p)
		.collect()
}
