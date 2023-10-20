/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::hash_map::{Entry, HashMap};
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;
use std::ptr;

use dunce::canonicalize;
use mozjs::jsapi::JSObject;
use url::Url;

use ion::{Context, Error, Object, Value};
use ion::exception::ThrowException;
use ion::module::{Module, ModuleData, ModuleLoader, ModuleRequest};

use crate::cache::locate_in_cache;
use crate::cache::map::save_sourcemap;
use crate::config::Config;

#[derive(Default)]
pub struct Loader {
	registry: HashMap<String, *mut JSObject>,
}

impl ModuleLoader for Loader {
	fn resolve(&mut self, cx: &Context, private: &Value, request: &ModuleRequest) -> *mut JSObject {
		let specifier = request.specifier(cx).to_owned(cx);
		let data = ModuleData::from_private(cx, private);

		let path = if specifier.starts_with("./") || specifier.starts_with("../") {
			Path::new(data.as_ref().and_then(|d| d.path.as_ref()).unwrap())
				.parent()
				.unwrap()
				.join(&specifier)
		} else {
			Path::new(&specifier).to_path_buf()
		};

		let str = String::from(path.to_str().unwrap());
		self.registry
			.get(&str)
			.copied()
			.or_else(|| {
				if let Ok(script) = read_to_string(&path) {
					let is_typescript = Config::global().typescript && path.extension() == Some(OsStr::new("ts"));
					let (script, sourcemap) = is_typescript
						.then(|| locate_in_cache(&path, &script))
						.flatten()
						.map(|(s, sm)| (s, Some(sm)))
						.unwrap_or_else(|| (script, None));
					if let Some(sourcemap) = sourcemap {
						save_sourcemap(&path, sourcemap);
					}

					let module = Module::compile(cx, &specifier, Some(path.as_path()), &script);

					if let Ok((module, _)) = module {
						let request = ModuleRequest::new(cx, path.to_str().unwrap());
						Some(self.register(cx, module.0.handle().get(), &request))
					} else {
						Error::new(&format!("Unable to compile module: {}\0", specifier), None).throw(cx);
						None
					}
				} else {
					Error::new(&format!("Unable to read module: {}", specifier), None).throw(cx);
					None
				}
			})
			.unwrap_or_else(ptr::null_mut)
	}

	fn register(&mut self, cx: &Context, module: *mut JSObject, request: &ModuleRequest) -> *mut JSObject {
		let specifier = request.specifier(cx).to_owned(cx);
		match self.registry.entry(specifier) {
			Entry::Vacant(v) => *v.insert(module),
			Entry::Occupied(_) => ptr::null_mut(),
		}
	}

	fn metadata(&self, cx: &Context, private: &Value, meta: &mut Object) -> bool {
		let data = ModuleData::from_private(cx, private);

		if let Some(data) = data {
			if let Some(path) = data.path.as_ref() {
				let url = Url::from_file_path(canonicalize(path).unwrap()).unwrap();
				if !meta.set_as(cx, "url", url.as_str()) {
					return false;
				}
			}
		}
		true
	}
}
