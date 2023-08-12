/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::Path;
use std::ptr;

use mozjs::jsapi::{
	CreateModuleRequest, GetModuleRequestSpecifier, Handle, JS_GetRuntime, JSContext, JSObject, SetModuleMetadataHook, SetModuleResolveHook,
};
use mozjs::jsval::JSVal;
use mozjs::rust::{CompileOptionsWrapper, transform_u16_to_source_text};
use mozjs_sys::jsapi::JS::{CompileModule, ModuleEvaluate, ModuleLink, ReadOnlyCompileOptions, SetModulePrivate};

use crate::{Context, ErrorReport, Local, Object, Promise, Value};
use crate::conversions::{FromValue, ToValue};

#[derive(Clone, Debug)]
pub struct ModuleData {
	pub path: Option<String>,
}

impl ModuleData {
	pub fn from_private<'cx: 'v, 'v>(cx: &'cx Context, private: &Value<'v>) -> Option<ModuleData> {
		private.is_object().then(|| {
			let private = private.to_object(cx);
			let path: Option<String> = private.get_as(cx, "path", true, ());
			ModuleData { path }
		})
	}

	pub fn to_object<'cx>(&self, cx: &'cx Context) -> Object<'cx> {
		let mut object = Object::new(cx);
		object.set_as(cx, "path", &self.path);
		object
	}
}

#[derive(Debug)]
pub struct ModuleRequest<'r>(Object<'r>);

impl<'r> ModuleRequest<'r> {
	pub fn new<S: AsRef<str>>(cx: &'r Context, specifier: S) -> ModuleRequest<'r> {
		let specifier = crate::String::new(cx, specifier.as_ref()).unwrap();
		ModuleRequest(cx.root_object(unsafe { CreateModuleRequest(**cx, specifier.handle().into()) }).into())
	}

	pub unsafe fn from_raw_request(request: Handle<*mut JSObject>) -> ModuleRequest<'r> {
		ModuleRequest(Object::from(Local::from_raw_handle(request)))
	}

	pub fn specifier<'cx>(&self, cx: &'cx Context) -> crate::String<'cx> {
		cx.root_string(unsafe { GetModuleRequestSpecifier(**cx, self.0.handle().into()) }).into()
	}
}

#[derive(Clone, Debug)]
pub struct ModuleError {
	pub kind: ModuleErrorKind,
	pub report: ErrorReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleErrorKind {
	Compilation,
	Instantiation,
	Evaluation,
}

impl ModuleError {
	fn new(report: ErrorReport, kind: ModuleErrorKind) -> ModuleError {
		ModuleError { kind, report }
	}

	pub fn format(&self, cx: &Context) -> String {
		self.report.format(cx)
	}
}

#[derive(Debug)]
pub struct Module<'m>(pub Object<'m>);

impl<'cx> Module<'cx> {
	#[allow(clippy::result_large_err)]
	pub fn compile(cx: &'cx Context, filename: &str, path: Option<&Path>, script: &str) -> Result<(Module<'cx>, Option<Promise<'cx>>), ModuleError> {
		let script: Vec<u16> = script.encode_utf16().collect();
		let mut source = transform_u16_to_source_text(script.as_slice());
		let filename = path.and_then(Path::to_str).unwrap_or(filename);
		let options = unsafe { CompileOptionsWrapper::new(**cx, filename, 1) };

		let module = unsafe { CompileModule(**cx, options.ptr as *const ReadOnlyCompileOptions, &mut source) };

		if !module.is_null() {
			let module = Module(Object::from(cx.root_object(module)));

			let data = ModuleData {
				path: path.and_then(Path::to_str).map(String::from),
			};

			unsafe {
				let private = data.to_object(cx).as_value(cx);
				SetModulePrivate(**module.0, &**private);
			}

			if let Err(error) = module.instantiate(cx) {
				return Err(ModuleError::new(error, ModuleErrorKind::Instantiation));
			}

			let eval_result = module.evaluate(cx);
			match eval_result {
				Ok(val) => unsafe {
					let promise = Promise::from_value(cx, &val, true, ()).ok();
					Ok((module, promise))
				},
				Err(error) => Err(ModuleError::new(error, ModuleErrorKind::Evaluation)),
			}
		} else {
			Err(ModuleError::new(ErrorReport::new(cx).unwrap(), ModuleErrorKind::Compilation))
		}
	}

	pub fn instantiate(&self, cx: &Context) -> Result<(), ErrorReport> {
		if unsafe { ModuleLink(**cx, self.0.handle().into()) } {
			Ok(())
		} else {
			Err(ErrorReport::new(cx).unwrap())
		}
	}

	pub fn evaluate(&self, cx: &'cx Context) -> Result<Value<'cx>, ErrorReport> {
		let mut rval = Value::undefined(cx);
		if unsafe { ModuleEvaluate(**cx, self.0.handle().into(), rval.handle_mut().into()) } {
			Ok(rval)
		} else {
			Err(ErrorReport::new_with_exception_stack(cx).unwrap())
		}
	}
}

pub trait ModuleLoader {
	fn resolve<'cx: 'p + 'r, 'p, 'r>(&mut self, cx: &'cx Context, private: &Value<'p>, request: &ModuleRequest<'r>) -> *mut JSObject;

	fn register<'cx: 'r, 'r>(&mut self, cx: &'cx Context, module: *mut JSObject, request: &ModuleRequest<'r>) -> *mut JSObject;

	fn metadata<'cx: 'p + 'm, 'p, 'm>(&self, cx: &'cx Context, private: &Value<'p>, meta: &mut Object<'m>) -> bool;
}

pub fn init_module_loader<ML: ModuleLoader + 'static>(cx: &Context, loader: ML) {
	unsafe extern "C" fn resolve(mut cx: *mut JSContext, private: Handle<JSVal>, request: Handle<*mut JSObject>) -> *mut JSObject {
		let cx = Context::new(&mut cx);

		let loader = unsafe { &mut (*cx.get_private()).module_loader };
		loader
			.as_mut()
			.map(|loader| {
				let private = Value::from(Local::from_raw_handle(private));
				let request = unsafe { ModuleRequest::from_raw_request(request) };
				(**loader).resolve(&cx, &private, &request)
			})
			.unwrap_or_else(ptr::null_mut)
	}

	unsafe extern "C" fn metadata(mut cx: *mut JSContext, private_data: Handle<JSVal>, metadata: Handle<*mut JSObject>) -> bool {
		let cx = Context::new(&mut cx);

		let loader = unsafe { &mut (*cx.get_private()).module_loader };
		loader
			.as_mut()
			.map(|loader| {
				let private = Value::from(Local::from_raw_handle(private_data));
				let mut metadata = Object::from(Local::from_raw_handle(metadata));
				(**loader).metadata(&cx, &private, &mut metadata)
			})
			.unwrap_or_else(|| true)
	}

	unsafe {
		(*cx.get_private()).module_loader = Some(Box::into_raw(Box::new(loader)));

		let rt = JS_GetRuntime(**cx);
		SetModuleResolveHook(rt, Some(resolve));
		SetModuleMetadataHook(rt, Some(metadata));
	}
}
