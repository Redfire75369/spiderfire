/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::collections::hash_map::{Entry, HashMap};
use std::fs::read_to_string;
use std::path::Path;
use std::ptr;

use dunce::canonicalize;
use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::{
	CompileModule, Handle, JS_GetRuntime, JSString, ModuleEvaluate, ModuleInstantiate, ReadOnlyCompileOptions, SetModuleMetadataHook,
	SetModulePrivate, SetModuleResolveHook, Value,
};
use mozjs::jsval::UndefinedValue;
use mozjs::rust::{CompileOptionsWrapper, transform_u16_to_source_text};
use url::Url;

use ion::exception::{ErrorReport, Exception};
use ion::IonContext;
use ion::objects::object::{IonObject, IonRawObject};
use ion::types::string::from_string;

thread_local!(static MODULE_REGISTRY: RefCell<HashMap<String, IonModule>> = RefCell::new(HashMap::new()));

#[derive(Clone, Debug)]
pub struct ModuleData {
	pub path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct IonModule {
	module: IonObject,
	#[allow(dead_code)]
	data: ModuleData,
}

impl ModuleData {
	fn from_module(cx: IonContext, module: Handle<Value>) -> Option<ModuleData> {
		if module.get().is_object() {
			let obj = IonObject::from(module.get().to_object());
			let path = unsafe { obj.get_as::<String>(cx, "path", ()) };

			Some(ModuleData { path })
		} else {
			None
		}
	}

	pub unsafe fn to_object(&self, cx: IonContext) -> IonObject {
		let mut data = IonObject::new(cx);

		if let Some(path) = self.path.as_ref() {
			data.set_as(cx, "path", path);
		} else {
			data.set_as(cx, "path", ());
		}

		data
	}
}

impl IonModule {
	pub fn compile(cx: IonContext, filename: &str, path: Option<&Path>, script: &str) -> Result<IonModule, ErrorReport> {
		let script: Vec<u16> = script.encode_utf16().collect();
		let mut source = transform_u16_to_source_text(script.as_slice());
		let options = unsafe { CompileOptionsWrapper::new(cx, filename, 1) };

		let module = unsafe { CompileModule(cx, options.ptr as *const ReadOnlyCompileOptions, &mut source) };
		rooted!(in(cx) let rooted_module = module);

		unsafe {
			if !rooted_module.is_null() {
				let data = if let Some(path) = path {
					ModuleData {
						path: if let Some(path_str) = path.to_str() {
							Some(String::from(path_str))
						} else {
							None
						},
					}
				} else {
					ModuleData { path: None }
				};
				SetModulePrivate(module, &data.to_object(cx).to_value());

				let module = IonModule {
					module: IonObject::from(module),
					data,
				};

				if let Err(e) = module.instantiate(cx) {
					eprintln!("Module Instantiation Failure");
					return Err(ErrorReport::new(e));
				}

				if let Err(e) = module.evaluate(cx) {
					eprintln!("Module Evaluation Failure");
					return Err(ErrorReport::new(e));
				}

				Ok(module)
			} else {
				let exception = Exception::new(cx).unwrap();
				Err(ErrorReport::new(exception))
			}
		}
	}

	pub unsafe fn instantiate(&self, cx: IonContext) -> Result<(), Exception> {
		rooted!(in(cx) let rooted_module = self.module.raw());
		if ModuleInstantiate(cx, rooted_module.handle().into()) {
			Ok(())
		} else {
			Err(Exception::new(cx).unwrap())
		}
	}

	pub unsafe fn evaluate(&self, cx: IonContext) -> Result<Value, Exception> {
		rooted!(in(cx) let rooted_module = self.module.raw());
		rooted!(in(cx) let mut rval = UndefinedValue());
		if ModuleEvaluate(cx, rooted_module.handle().into(), rval.handle_mut().into()) {
			Ok(rval.get())
		} else {
			Err(Exception::new(cx).unwrap())
		}
	}

	pub fn register(&self, name: &str) -> bool {
		MODULE_REGISTRY.with(|registry| {
			let mut registry = registry.borrow_mut();
			match (*registry).entry(String::from(name)) {
				Entry::Vacant(v) => {
					v.insert(self.clone());
					true
				}
				Entry::Occupied(_) => false,
			}
		})
	}

	pub fn resolve(cx: IonContext, specifier: &str, data: ModuleData) -> Option<IonModule> {
		let path = if specifier.starts_with("./") || specifier.starts_with("../") {
			Path::new(&data.path.unwrap()).parent().unwrap().join(specifier)
		} else if specifier.starts_with('/') {
			Path::new(specifier).to_path_buf()
		} else {
			Path::new(specifier).to_path_buf()
		};

		let opt = MODULE_REGISTRY.with(|registry| {
			let mut registry = registry.borrow_mut();

			let str = String::from(path.to_str().unwrap());
			match (*registry).entry(str) {
				Entry::Vacant(_) => None,
				Entry::Occupied(o) => Some(o.get().clone()),
			}
		});

		if opt.is_some() {
			opt
		} else {
			if let Ok(script) = read_to_string(&path) {
				let module = IonModule::compile(cx, specifier, Some(path.as_path()), &script);
				if let Ok(module) = module {
					module.register(path.to_str().unwrap());
					Some(module)
				} else {
					eprintln!("Module Compilation Failed");
					None
				}
			} else {
				eprintln!("Module Read Failed");
				None
			}
		}
	}
}

pub unsafe extern "C" fn resolve_module(cx: IonContext, private_data: Handle<Value>, specifier: Handle<*mut JSString>) -> IonRawObject {
	let specifier = jsstr_to_string(cx, specifier.get());
	let data = ModuleData::from_module(cx, private_data).unwrap();

	if let Some(module) = IonModule::resolve(cx, &specifier, data) {
		module.module.raw()
	} else {
		ptr::null_mut()
	}
}

pub unsafe extern "C" fn module_metadata(cx: IonContext, private_data: Handle<Value>, meta: Handle<IonRawObject>) -> bool {
	let data = ModuleData::from_module(cx, private_data).unwrap();

	if let Some(path) = data.path.as_ref() {
		let url = Url::from_file_path(canonicalize(path).unwrap()).unwrap();
		let mut meta = IonObject::from(meta.get());
		if !meta.set(cx, "url", from_string(cx, url.as_str())) {
			return false;
		}
	}

	true
}

pub fn init_module_loaders(cx: IonContext) {
	unsafe {
		SetModuleResolveHook(JS_GetRuntime(cx), Some(resolve_module));
		SetModuleMetadataHook(JS_GetRuntime(cx), Some(module_metadata));
	}
}
