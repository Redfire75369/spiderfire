/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::std::cell::RefCell;
use ::std::collections::hash_map::{Entry, HashMap};
use ::std::fs::read_to_string;
use ::std::path::Path;
use ::std::ptr;

use mozjs::conversions::{jsstr_to_string, ToJSValConvertible};
use mozjs::jsapi::*;
use mozjs::jsval::{BooleanValue, UndefinedValue};
use mozjs::rust::{CompileOptionsWrapper, transform_u16_to_source_text};

use ion::exceptions::exception::report_and_clear_exception;
use ion::functions::macros::IonContext;
use ion::objects::object::IonObject;

thread_local! {
	static MODULE_REGISTRY: RefCell<HashMap<String, *mut JSObject>> = RefCell::new(HashMap::new());
}

#[derive(Clone, Debug)]
struct ModuleData {
	pub path: Option<String>,
	pub std: bool,
}

unsafe fn get_module_data(cx: IonContext, module: Handle<Value>) -> Option<ModuleData> {
	if module.get().is_object() && !module.get().is_null() {
		let obj = IonObject::from_value(module.get());
		let path = obj.get_as::<String>(cx, String::from("path"), ());
		let std = obj.get_as::<bool>(cx, String::from("std"), ()).unwrap();

		Some(ModuleData { path, std })
	} else {
		None
	}
}

pub unsafe fn compile_module(cx: IonContext, filename: &str, path: Option<&Path>, script: &str) -> Option<*mut JSObject> {
	let options = CompileOptionsWrapper::new(cx, filename, 1);
	let script_text: Vec<u16> = script.encode_utf16().collect();
	let mut source = transform_u16_to_source_text(script_text.as_slice());

	let module = CompileModule(cx, options.ptr as *const ReadOnlyCompileOptions, &mut source);
	rooted!(in(cx) let rooted_module = module);
	if module.is_null() {
		report_and_clear_exception(cx);
		return None;
	}

	if let Some(path) = path {
		if let Some(path_str) = path.to_str() {
			rooted!(in(cx) let mut path = UndefinedValue());
			path_str.to_jsval(cx, path.handle_mut().into());
			let mut data = IonObject::new(cx);
			data.set(cx, String::from("path"), path.get());
			data.set(cx, String::from("std"), BooleanValue(false));
			SetModulePrivate(module, &data.to_value());
		}
	} else {
		let mut data = IonObject::new(cx);
		data.set(cx, String::from("path"), UndefinedValue());
		data.set(cx, String::from("std"), BooleanValue(true));
		SetModulePrivate(module, &data.to_value());
	}

	if !ModuleInstantiate(cx, rooted_module.handle().into()) {
		eprintln!("Failed to instantiate module :(");
		report_and_clear_exception(cx);
		return None;
	}

	rooted!(in (cx) let mut rval = UndefinedValue());
	if !ModuleEvaluate(cx, rooted_module.handle().into(), rval.handle_mut().into()) {
		eprintln!("Failed to evaluate module :(");
		report_and_clear_exception(cx);
		return None;
	}

	Some(module)
}

pub fn register_module(cx: IonContext, name: &str, module: *mut JSObject) -> bool {
	MODULE_REGISTRY.with(|registry| {
		let mut registry = registry.borrow_mut();
		rooted!(in(cx) let mut module = module);
		match (*registry).entry(name.to_string()) {
			Entry::Vacant(v) => {
				v.insert(module.handle().get());
				true
			}
			Entry::Occupied(_) => false,
		}
	})
}

pub unsafe extern "C" fn resolve_module(cx: IonContext, module_private: Handle<Value>, name: Handle<*mut JSString>) -> *mut JSObject {
	let name = jsstr_to_string(cx, name.get());
	let data = get_module_data(cx, module_private);

	let path = if name.starts_with("./") || name.starts_with("../") {
		Path::new(&data.clone().unwrap().path.unwrap()).parent().unwrap().join(&name)
	} else if name.starts_with("/") {
		Path::new(&name).to_path_buf()
	} else {
		Path::new(&name).to_path_buf()
	};

	let path_str = if let Some(p) = path.to_str() {
		String::from(p)
	} else {
		return ptr::null_mut();
	};

	let opt = MODULE_REGISTRY.with(|registry| {
		let mut registry = registry.borrow_mut();
		match (*registry).entry(path_str.clone()) {
			Entry::Vacant(_) => None,
			Entry::Occupied(o) => Some(*o.get()),
		}
	});
	if let Some(module) = opt {
		return module;
	}

	if let Ok(script) = read_to_string(&path) {
		let module = compile_module(cx, &name, Some(&path), &script);
		if let Some(module) = module {
			register_module(cx, &path_str, module);
			module
		} else {
			eprintln!("Module could not be compiled :(");
			ptr::null_mut()
		}
	} else {
		eprintln!("Could not read module :(");
		ptr::null_mut()
	}
}
