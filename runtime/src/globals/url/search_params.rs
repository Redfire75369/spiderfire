/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */
use form_urlencoded::{parse, Serializer};
use mozjs::jsapi::{Heap, JSObject};

use ion::{ClassDefinition, Context, Error, ErrorKind, JSIterator, Local, Object, OwnedKey, Result, Value};
use ion::class::Reflector;
use ion::conversions::{FromValue, ToValue};
use ion::symbol::WellKnownSymbolCode;

use crate::globals::url::URL;

pub struct URLSearchParamsInit(Vec<(String, String)>);

impl<'cx> FromValue<'cx> for URLSearchParamsInit {
	type Config = ();
	fn from_value(cx: &'cx Context, value: &Value, strict: bool, _: ()) -> Result<URLSearchParamsInit> {
		if let Ok(vec) = <Vec<Value>>::from_value(cx, value, strict, ()) {
			let entries = vec
				.iter()
				.map(|value| {
					let vec = <Vec<String>>::from_value(cx, value, strict, ())?;
					let boxed: Box<[String; 2]> = vec
						.try_into()
						.map_err(|_| Error::new("Expected Search Parameter Entry with Length 2", ErrorKind::Type))?;
					let [name, value] = *boxed;
					Ok((name, value))
				})
				.collect::<Result<_>>()?;
			Ok(URLSearchParamsInit(entries))
		} else if let Ok(object) = Object::from_value(cx, value, strict, ()) {
			let vec = object
				.iter(cx, None)
				.map(|(key, value)| {
					let value = String::from_value(cx, &value, strict, ())?;
					match key.to_owned_key(cx) {
						OwnedKey::Int(i) => Ok((i.to_string(), value)),
						OwnedKey::String(key) => Ok((key, value)),
						_ => unreachable!(),
					}
				})
				.collect::<Result<_>>()?;
			Ok(URLSearchParamsInit(vec))
		} else if let Ok(string) = String::from_value(cx, value, strict, ()) {
			Ok(URLSearchParamsInit(parse(string.as_bytes()).into_owned().collect()))
		} else {
			Err(Error::new("Invalid Search Params Initialiser", ErrorKind::Type))
		}
	}
}

#[js_class]
pub struct URLSearchParams {
	reflector: Reflector,
	pairs: Vec<(String, String)>,
	pub(super) url: Option<Heap<*mut JSObject>>,
}

#[js_class]
impl URLSearchParams {
	#[ion(constructor)]
	pub fn constructor(init: Option<URLSearchParamsInit>) -> URLSearchParams {
		let pairs = init.map(|init| init.0).unwrap_or_default();
		URLSearchParams {
			reflector: Reflector::default(),
			pairs,
			url: None,
		}
	}

	pub(super) fn new(pairs: Vec<(String, String)>) -> URLSearchParams {
		URLSearchParams {
			reflector: Reflector::default(),
			pairs,
			url: Some(Heap::default()),
		}
	}

	pub(super) fn set_pairs(&mut self, pairs: Vec<(String, String)>) {
		self.pairs = pairs;
	}

	#[ion(get)]
	pub fn get_size(&self) -> i32 {
		self.pairs.len() as i32
	}

	pub fn append(&mut self, name: String, value: String) {
		self.pairs.push((name, value));
		self.update();
	}

	pub fn delete(&mut self, name: String, value: Option<String>) {
		if let Some(value) = value {
			self.pairs.retain(|(k, v)| k != &name && v != &value)
		} else {
			self.pairs.retain(|(k, _)| k != &name)
		}
		self.update();
	}

	pub fn get(&self, key: String) -> Option<String> {
		self.pairs.iter().find(|(k, _)| k == &key).map(|(_, v)| v.clone())
	}

	#[ion(name = "get_all")]
	pub fn get_all(&self, key: String) -> Vec<String> {
		self.pairs.iter().filter(|(k, _)| k == &key).map(|(_, v)| v.clone()).collect()
	}

	pub fn has(&self, key: String, value: Option<String>) -> bool {
		if let Some(value) = value {
			self.pairs.iter().any(|(k, v)| k == &key && v == &value)
		} else {
			self.pairs.iter().any(|(k, _)| k == &key)
		}
	}

	pub fn set(&mut self, key: String, value: String) {
		let mut i = 0;
		let mut index = None;

		self.pairs.retain(|(k, _)| {
			if index.is_none() {
				if k == &key {
					index = Some(i);
				} else {
					i += 1;
				}
				true
			} else {
				k != &key
			}
		});

		match index {
			Some(index) => self.pairs[index].1 = value,
			None => self.pairs.push((key, value)),
		}

		self.update()
	}

	pub fn sort(&mut self) {
		self.pairs.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
		self.update()
	}

	#[ion(name = "toString")]
	#[allow(clippy::inherent_to_string)]
	pub fn to_string(&self) -> String {
		Serializer::new(String::new()).extend_pairs(&*self.pairs).finish()
	}

	fn update(&mut self) {
		if let Some(url) = &self.url {
			let mut url = Object::from(unsafe { Local::from_heap(url) });
			let url = URL::get_mut_private(&mut url);
			if self.pairs.is_empty() {
				url.url.set_query(None);
			} else {
				url.url.query_pairs_mut().clear().extend_pairs(&self.pairs);
			}
		}
	}

	#[ion(name = WellKnownSymbolCode::Iterator)]
	pub fn iterator(cx: &Context, #[ion(this)] this: &Object) -> ion::Iterator {
		let thisv = this.as_value(cx);
		ion::Iterator::new(SearchParamsIterator::default(), &thisv)
	}
}

#[derive(Default)]
pub struct SearchParamsIterator(usize);

impl JSIterator for SearchParamsIterator {
	fn next_value<'cx>(&mut self, cx: &'cx Context, private: &Value<'cx>) -> Option<Value<'cx>> {
		let object = private.to_object(cx);
		let search_params = URLSearchParams::get_private(&object);
		let pair = search_params.pairs.get(self.0);
		pair.map(move |(k, v)| {
			self.0 += 1;
			[k, v].as_value(cx)
		})
	}
}
