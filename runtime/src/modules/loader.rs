/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::collections::hash_map::{Entry, HashMap};
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;
use std::ptr;

use dunce::canonicalize;
use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{
	CompileModule, Handle, JS_GetRuntime, JS_ReportErrorUTF8, JSObject, JSString, ModuleEvaluate, ModuleInstantiate, ReadOnlyCompileOptions,
	SetModuleMetadataHook, SetModulePrivate, SetModuleResolveHook,
};
use mozjs::jsval::{JSVal, UndefinedValue};
use mozjs::rust::{CompileOptionsWrapper, transform_u16_to_source_text};
use url::Url;

use ion::{Context, ErrorReport, Exception, Object, Promise};

use crate::cache::locate_in_cache;
use crate::cache::map::save_sourcemap;
use crate::config::Config;
use crate::modules::{ModuleError, ModuleErrorKind};

thread_local!(static MODULE_REGISTRY: RefCell<HashMap<String, Module>> = RefCell::new(HashMap::new()));

#[derive(Clone, Debug)]
pub struct ModuleData {
	pub path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Module {
	module: Object,
	#[allow(dead_code)]
	data: ModuleData,
}

impl ModuleData {
	fn from_module(cx: Context, module: Handle<JSVal>) -> Option<ModuleData> {
		if module.get().is_object() {
			let obj = Object::from(module.get().to_object());
			let path = obj.get_as::<String>(cx, "path", ());

			Some(ModuleData { path })
		} else {
			None
		}
	}

	pub fn to_object(&self, cx: Context) -> Object {
		let mut data = Object::new(cx);

		if let Some(path) = self.path.as_ref() {
			data.set_as(cx, "path", path);
		} else {
			data.set_as(cx, "path", ());
		}

		data
	}
}

impl Module {
	pub fn compile(cx: Context, filename: &str, path: Option<&Path>, script: &str) -> Result<(Module, Option<Promise>), ModuleError> {
		let script: Vec<u16> = script.encode_utf16().collect();
		let mut source = transform_u16_to_source_text(script.as_slice());
		let filename = path.and_then(Path::to_str).unwrap_or(filename);
		let options = unsafe { CompileOptionsWrapper::new(cx, filename, 1) };

		let module = unsafe { CompileModule(cx, options.ptr as *const ReadOnlyCompileOptions, &mut source) };
		rooted!(in(cx) let rooted_module = module);

		unsafe {
			if !rooted_module.is_null() {
				let data = ModuleData {
					path: path.and_then(Path::to_str).map(String::from),
				};
				SetModulePrivate(module, &data.to_object(cx).to_value());

				let module = Module { module: Object::from(module), data };

				if let Err(exception) = module.instantiate(cx) {
					return Err(ModuleError::new(ErrorReport::new(exception), ModuleErrorKind::Instantiation));
				}

				let eval_result = module.evaluate(cx);
				match eval_result {
					Ok(val) => {
						let promise = Promise::from_value(cx, val);
						Ok((module, promise))
					}
					Err(exception) => Err(ModuleError::new(ErrorReport::new(exception), ModuleErrorKind::Evaluation)),
				}
			} else {
				let exception = Exception::new(cx).unwrap();
				Err(ModuleError::new(ErrorReport::new(exception), ModuleErrorKind::Compilation))
			}
		}
	}

	pub fn instantiate(&self, cx: Context) -> Result<(), Exception> {
		rooted!(in(cx) let rooted_module = *self.module);
		if unsafe { ModuleInstantiate(cx, rooted_module.handle().into()) } {
			Ok(())
		} else {
			Err(Exception::new(cx).unwrap())
		}
	}

	pub fn evaluate(&self, cx: Context) -> Result<JSVal, Exception> {
		rooted!(in(cx) let rooted_module = *self.module);
		rooted!(in(cx) let mut rval = UndefinedValue());
		if unsafe { ModuleEvaluate(cx, rooted_module.handle().into(), rval.handle_mut().into()) } {
			Ok(rval.get())
		} else {
			Err(Exception::new(cx).unwrap())
		}
	}

	pub fn register(self, name: &str) -> bool {
		MODULE_REGISTRY.with(|registry| {
			let mut registry = registry.borrow_mut();
			match (*registry).entry(String::from(name)) {
				Entry::Vacant(v) => {
					v.insert(self);
					true
				}
				Entry::Occupied(_) => false,
			}
		})
	}

	pub fn resolve(cx: Context, specifier: &str, data: ModuleData) -> Option<Module> {
		let path = if specifier.starts_with("./") || specifier.starts_with("../") {
			Path::new(data.path.as_ref().unwrap()).parent().unwrap().join(specifier)
		} else {
			Path::new(specifier).to_path_buf()
		};

		let module = MODULE_REGISTRY.with(|registry| {
			let mut registry = registry.borrow_mut();

			let str = String::from(path.to_str().unwrap());
			match (*registry).entry(str) {
				Entry::Occupied(o) => Some(o.get().clone()),
				Entry::Vacant(_) => None,
			}
		});

		module.or_else(|| {
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

				let module = Module::compile(cx, specifier, Some(path.as_path()), &script);

				if let Ok(module) = module {
					module.0.clone().register(path.to_str().unwrap());
					Some(module.0)
				} else {
					unsafe {
						JS_ReportErrorUTF8(cx, format!("Unable to compile module: {}\0", specifier).as_ptr() as *const i8);
					}
					None
				}
			} else {
				unsafe {
					JS_ReportErrorUTF8(cx, format!("Unable to read module: {}\0", specifier).as_ptr() as *const i8);
				}
				None
			}
		})
	}
}

pub unsafe extern "C" fn resolve_module(cx: Context, private_data: Handle<JSVal>, specifier: Handle<*mut JSString>) -> *mut JSObject {
	let specifier = jsstr_to_string(cx, specifier.get());
	let data = ModuleData::from_module(cx, private_data).unwrap();

	Module::resolve(cx, &specifier, data)
		.map(|module| *module.module)
		.unwrap_or(ptr::null_mut())
}

pub unsafe extern "C" fn module_metadata(cx: Context, private_data: Handle<JSVal>, meta: Handle<*mut JSObject>) -> bool {
	let data = ModuleData::from_module(cx, private_data).unwrap();

	if let Some(path) = data.path.as_ref() {
		let url = Url::from_file_path(canonicalize(path).unwrap()).unwrap();
		let mut meta = Object::from(meta.get());
		if !meta.set_as(cx, "url", String::from(url.as_str())) {
			return false;
		}
	}

	true
}

pub fn init_module_loaders(cx: Context) {
	unsafe {
		SetModuleResolveHook(JS_GetRuntime(cx), Some(resolve_module));
		SetModuleMetadataHook(JS_GetRuntime(cx), Some(module_metadata));
	}
}
