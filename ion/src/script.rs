/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::{Compile, JSScript, Value};
use mozjs::jsval::UndefinedValue;
use mozjs::rust::{CompileOptionsWrapper, transform_u16_to_source_text};
use mozjs_sys::jsapi::JS_ExecuteScript;

use crate::exception::{ErrorReport, Exception};
use crate::IonContext;

pub type IonRawScript = *mut JSScript;

#[derive(Clone, Copy, Debug)]
pub struct IonScript {
	script: IonRawScript,
}

impl IonScript {
	pub fn compile(cx: IonContext, filename: &str, script: &str) -> Result<IonScript, Exception> {
		let script: Vec<u16> = script.encode_utf16().collect();
		let mut source = transform_u16_to_source_text(script.as_slice());
		let options = unsafe { CompileOptionsWrapper::new(cx, filename, 1) };

		let script = unsafe { Compile(cx, options.ptr, &mut source) };
		rooted!(in(cx) let rooted_script = script);

		if !rooted_script.is_null() {
			Ok(IonScript { script })
		} else {
			Err(unsafe { Exception::new(cx).unwrap() })
		}
	}

	pub fn evaluate(&self, cx: IonContext) -> Result<Value, ErrorReport> {
		rooted!(in(cx) let script = self.script);
		rooted!(in(cx) let mut rval = UndefinedValue());

		if unsafe { JS_ExecuteScript(cx, script.handle().into(), rval.handle_mut().into()) } {
			Ok(rval.get())
		} else {
			Err(unsafe { ErrorReport::new(Exception::new(cx).unwrap()) })
		}
	}

	pub fn compile_and_evaluate(cx: IonContext, filename: &str, script: &str) -> Result<Value, ErrorReport> {
		match IonScript::compile(cx, filename, script) {
			Ok(s) => match s.evaluate(cx) {
				Ok(v) => Ok(v),
				Err(e) => Err(e),
			},
			Err(e) => Err(ErrorReport::new(e)),
		}
	}
}
