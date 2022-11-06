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
	CompileModule, Handle, JS_GetRuntime, JS_SetProperty, JSContext, JSObject, JSString, ModuleEvaluate, ModuleInstantiate, ReadOnlyCompileOptions,
	SetModuleMetadataHook, SetModulePrivate, SetModuleResolveHook,
};
use mozjs::jsval::JSVal;
use mozjs::rust::{CompileOptionsWrapper, transform_u16_to_source_text};
use url::Url;

use ion::{Context, Error, ErrorReport, Object, Promise, Value};
use ion::conversions::{FromValue, ToValue};
use ion::exception::ThrowException;

use crate::cache::locate_in_cache;
use crate::cache::map::save_sourcemap;
use crate::config::Config;
use crate::modules::{ModuleError, ModuleErrorKind};

thread_local!(static MODULE_REGISTRY: RefCell<HashMap<String, (*mut JSObject, ModuleData)>> = RefCell::new(HashMap::new()));

#[derive(Clone, Debug)]
pub struct ModuleData {
	pub path: Option<String>,
}

#[derive(Debug)]
pub struct Module<'cx> {
	module: Object<'cx>,
	#[allow(dead_code)]
	data: ModuleData,
}

impl ModuleData {
	fn from_module(cx: &Context, module: Handle<JSVal>) -> Option<ModuleData> {
		if module.get().is_object() {
			let module = Object::from(cx.root_object(module.get().to_object()));
			let path = module.get_as::<String>(cx, "path", true, ());

			Some(ModuleData { path })
		} else {
			None
		}
	}

	pub fn to_object<'cx>(&self, cx: &'cx Context) -> Object<'cx> {
		let mut data = Object::new(cx);

		if let Some(path) = self.path.as_ref() {
			data.set_as(cx, "path", path);
		} else {
			data.set_as(cx, "path", &());
		}

		data
	}
}

impl<'cx> Module<'cx> {
	pub fn compile(cx: &'cx Context, filename: &str, path: Option<&Path>, script: &str) -> Result<(Module<'cx>, Option<Promise<'cx>>), ModuleError> {
		let script: Vec<u16> = script.encode_utf16().collect();
		let mut source = transform_u16_to_source_text(script.as_slice());
		let filename = path.and_then(Path::to_str).unwrap_or(filename);
		let options = unsafe { CompileOptionsWrapper::new(**cx, filename, 1) };

		let module = unsafe { CompileModule(**cx, options.ptr as *const ReadOnlyCompileOptions, &mut source) };

		unsafe {
			if !module.is_null() {
				let module = Object::from(cx.root_object(module));

				let data = ModuleData {
					path: path.and_then(Path::to_str).map(String::from),
				};
				let private = data.to_object(cx).as_value(cx);
				SetModulePrivate(**module, &**private);

				let module = Module { module, data };

				if let Err(error) = module.instantiate(cx) {
					return Err(ModuleError::new(error, ModuleErrorKind::Instantiation));
				}

				let eval_result = module.evaluate(cx);
				match eval_result {
					Ok(val) => {
						let promise = Promise::from_value(cx, &val, true, ()).ok();
						Ok((module, promise))
					}
					Err(error) => Err(ModuleError::new(error, ModuleErrorKind::Evaluation)),
				}
			} else {
				Err(ModuleError::new(ErrorReport::new(cx).unwrap(), ModuleErrorKind::Compilation))
			}
		}
	}

	pub fn instantiate(&self, cx: &Context) -> Result<(), ErrorReport> {
		if unsafe { ModuleInstantiate(**cx, self.module.handle().into()) } {
			Ok(())
		} else {
			Err(ErrorReport::new(cx).unwrap())
		}
	}

	pub fn evaluate(&self, cx: &'cx Context) -> Result<Value<'cx>, ErrorReport> {
		let mut rval = Value::undefined(cx);
		if unsafe { ModuleEvaluate(**cx, self.module.handle().into(), rval.handle_mut().into()) } {
			Ok(rval)
		} else {
			Err(ErrorReport::new_with_exception_stack(cx).unwrap())
		}
	}

	pub fn register(&self, name: &str) -> bool {
		MODULE_REGISTRY.with(|registry| {
			let mut registry = registry.borrow_mut();
			match (*registry).entry(String::from(name)) {
				Entry::Vacant(v) => {
					v.insert((**self.module, self.data.clone()));
					true
				}
				Entry::Occupied(_) => false,
			}
		})
	}

	pub fn resolve(cx: &'cx Context, specifier: &str, data: ModuleData) -> Option<Module<'cx>> {
		let path = if specifier.starts_with("./") || specifier.starts_with("../") {
			Path::new(data.path.as_ref().unwrap()).parent().unwrap().join(specifier)
		} else {
			Path::new(specifier).to_path_buf()
		};

		let module = MODULE_REGISTRY.with(|registry| {
			let mut registry = registry.borrow_mut();

			let str = String::from(path.to_str().unwrap());
			match (*registry).entry(str) {
				Entry::Occupied(o) => Some(Module {
					module: Object::from(cx.root_object(o.get().0)),
					data: o.get().1.clone(),
				}),
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

				if let Ok((module, _)) = module {
					module.register(path.to_str().unwrap());
					Some(module)
				} else {
					Error::new(&format!("Unable to compile module: {}\0", specifier), None).throw(cx);
					None
				}
			} else {
				Error::new(&format!("Unable to read module: {}", specifier), None).throw(cx);
				None
			}
		})
	}
}

pub unsafe extern "C" fn resolve_module(mut cx: *mut JSContext, private_data: Handle<JSVal>, specifier: Handle<*mut JSString>) -> *mut JSObject {
	let cx = Context::new(&mut cx);

	let specifier = jsstr_to_string(*cx, specifier.get());
	let data = ModuleData::from_module(&cx, private_data).unwrap();

	Module::resolve(&cx, &specifier, data)
		.map(|module| **module.module)
		.unwrap_or(ptr::null_mut())
}

pub unsafe extern "C" fn module_metadata(mut cx: *mut JSContext, private_data: Handle<JSVal>, meta: Handle<*mut JSObject>) -> bool {
	let cx = Context::new(&mut cx);
	let data = ModuleData::from_module(&cx, private_data).unwrap();

	if let Some(path) = data.path.as_ref() {
		let url = Url::from_file_path(canonicalize(path).unwrap()).unwrap();
		let value = url.as_str().as_value(&cx);
		if !JS_SetProperty(*cx, meta, "url\0".as_ptr() as *const i8, value.handle().into()) {
			return false;
		}
	}

	true
}

pub fn init_module_loaders(cx: &Context) {
	unsafe {
		SetModuleResolveHook(JS_GetRuntime(**cx), Some(resolve_module));
		SetModuleMetadataHook(JS_GetRuntime(**cx), Some(module_metadata));
	}
}
