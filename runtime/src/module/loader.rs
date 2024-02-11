/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::hash_map::{Entry, HashMap};
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use dunce::canonicalize;
use mozjs::jsapi::JSObject;
use url::Url;

use ion::{Context, Error, Local, Object, Result, Value};
use ion::module::{Module, ModuleData, ModuleLoader, ModuleRequest};

use crate::cache::locate_in_cache;
use crate::cache::map::save_sourcemap;
use crate::config::Config;

#[derive(Default)]
pub struct Loader {
	registry: HashMap<String, *mut JSObject>,
}

impl ModuleLoader for Loader {
	fn resolve<'cx>(&mut self, cx: &'cx Context, private: &Value, request: &ModuleRequest) -> Result<Module<'cx>> {
		let specifier = request.specifier(cx).to_owned(cx).unwrap();
		let data = ModuleData::from_private(cx, private);

		let path = if specifier.starts_with("./") || specifier.starts_with("../") {
			Path::new(data.as_ref().and_then(|d| d.path.as_ref()).unwrap())
				.parent()
				.unwrap()
				.join(&specifier)
		} else {
			Path::new(&specifier).to_path_buf()
		};

		let specifier = String::from(path.to_str().unwrap());
		if let Some(module) = self.registry.get(&specifier) {
			Ok(Module(Object::from(unsafe { Local::from_marked(module) })))
		} else if let Ok(script) = read_to_string(&path) {
			let is_typescript = Config::global().typescript && path.extension() == Some(OsStr::new("ts"));
			let (script, sourcemap) = is_typescript
				.then(|| locate_in_cache(&path, &script))
				.flatten()
				.map(|(s, sm)| (s, Some(sm)))
				.unwrap_or_else(|| (script, None));
			if let Some(sourcemap) = sourcemap {
				save_sourcemap(&path, sourcemap);
			}

			let module = Module::compile_and_evaluate(cx, &specifier, Some(path.as_path()), &script);

			if let Ok((module, _)) = module {
				let request = ModuleRequest::new(cx, path.to_str().unwrap());
				self.register(cx, module.0.handle().get(), &request)?;
				Ok(module)
			} else {
				Err(Error::new(format!("Unable to compile module: {}\0", specifier), None))
			}
		} else {
			Err(Error::new(format!("Unable to read module: {}", specifier), None))
		}
	}

	fn register(&mut self, cx: &Context, module: *mut JSObject, request: &ModuleRequest) -> Result<()> {
		let specifier = request.specifier(cx).to_owned(cx)?;
		match self.registry.entry(specifier) {
			Entry::Vacant(v) => {
				v.insert(module);
				Ok(())
			}
			Entry::Occupied(_) => Err(Error::new("Module already exists", None)),
		}
	}

	fn metadata(&self, cx: &Context, private: &Value, meta: &Object) -> Result<()> {
		let data = ModuleData::from_private(cx, private);

		if let Some(data) = data {
			if let Some(path) = data.path.as_ref() {
				let url = Url::from_file_path(canonicalize(path)?).unwrap();
				if !meta.set_as(cx, "url", url.as_str()) {
					return Err(Error::none());
				}
			}
		}
		Ok(())
	}
}
