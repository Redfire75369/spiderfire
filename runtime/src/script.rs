/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use mozjs::jsapi::{Compile, JS_ExecuteScript, JSScript};
use mozjs::rust::{CompileOptionsWrapper, transform_u16_to_source_text};

use ion::{Context, ErrorReport, Local, Value};

#[derive(Clone, Copy, Debug)]
pub struct Script<'cx> {
	script: &'cx Local<'cx, *mut JSScript>,
}

impl<'cx> Script<'cx> {
	/// Compiles a script with a given filename and returns the compiled script.
	/// Returns [Err] when script compilation fails.
	pub fn compile(cx: &'cx Context, path: &Path, script: &str) -> Result<Script<'cx>, ErrorReport> {
		let script: Vec<u16> = script.encode_utf16().collect();
		let mut source = transform_u16_to_source_text(script.as_slice());
		let options = unsafe { CompileOptionsWrapper::new(**cx, path.to_str().unwrap(), 1) };

		let script = unsafe { Compile(**cx, options.ptr, &mut source) };

		if !script.is_null() {
			Ok(Script { script: cx.root_script(script) })
		} else {
			Err(ErrorReport::new_with_exception_stack(cx).unwrap())
		}
	}

	/// Evaluates a script and returns its return value.
	/// Returns [Err] when an exception occurs during script evaluation.
	pub fn evaluate(&self, cx: &'cx Context) -> Result<Value<'cx>, ErrorReport> {
		let mut rval = Value::undefined(cx);

		if unsafe { JS_ExecuteScript(**cx, self.script.handle().into(), rval.handle_mut().into()) } {
			Ok(rval)
		} else {
			Err(ErrorReport::new_with_exception_stack(cx).unwrap())
		}
	}

	/// Compiles and evaluates a script with a given filename, and returns its return value.
	/// Returns [Err] when script compilation fails or an exception occurs during script evaluation.
	pub fn compile_and_evaluate(cx: &'cx Context, path: &Path, script: &str) -> Result<Value<'cx>, ErrorReport> {
		match Script::compile(cx, path, script) {
			Ok(s) => match s.evaluate(cx) {
				Ok(v) => Ok(v),
				Err(e) => Err(e),
			},
			Err(e) => Err(e),
		}
	}
}
