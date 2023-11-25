/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{error, fmt, ptr};
use std::fmt::{Display, Formatter};

use mozjs::error::{throw_internal_error, throw_range_error, throw_type_error};
use mozjs::jsapi::{CreateError, JS_ReportErrorUTF8, JSExnType, JSObject, JSProtoKey, UndefinedHandleValue};

use crate::{Context, Object, Stack, Value};
use crate::conversions::ToValue;
use crate::exception::ThrowException;
use crate::stack::Location;

/// Represents the types of errors that can be thrown and are recognised in the JS Runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
	Normal,
	Internal,
	Aggregate,
	Eval,
	Range,
	Reference,
	Syntax,
	Type,
	WasmCompile,
	WasmLink,
	WasmRuntime,
	None,
}

impl ErrorKind {
	/// Converts a prototype key into an [ErrorKind].
	/// Returns [ErrorKind::None] for unrecognised prototype keys.
	pub fn from_proto_key(key: JSProtoKey) -> ErrorKind {
		use JSProtoKey::{
			JSProto_AggregateError, JSProto_CompileError, JSProto_Error, JSProto_EvalError, JSProto_InternalError,
			JSProto_LinkError, JSProto_RangeError, JSProto_ReferenceError, JSProto_RuntimeError, JSProto_SyntaxError,
			JSProto_TypeError,
		};
		use ErrorKind as EK;
		match key {
			JSProto_Error => EK::Normal,
			JSProto_InternalError => EK::Internal,
			JSProto_AggregateError => EK::Aggregate,
			JSProto_EvalError => EK::Eval,
			JSProto_RangeError => EK::Range,
			JSProto_ReferenceError => EK::Reference,
			JSProto_SyntaxError => EK::Syntax,
			JSProto_TypeError => EK::Type,
			JSProto_CompileError => EK::WasmCompile,
			JSProto_LinkError => EK::WasmLink,
			JSProto_RuntimeError => EK::WasmRuntime,
			_ => EK::None,
		}
	}

	/// Converts an [ErrorKind] into a an exception type.
	///
	/// Note that [`ErrorKind::None`] is converted to [`JSEXN_ERR`](JSExnType::JSEXN_ERR).
	pub fn to_exception_type(&self) -> JSExnType {
		use ErrorKind as EK;
		use JSExnType as JSET;
		match self {
			EK::Normal => JSET::JSEXN_ERR,
			EK::Internal => JSET::JSEXN_INTERNALERR,
			EK::Aggregate => JSET::JSEXN_AGGREGATEERR,
			EK::Eval => JSET::JSEXN_EVALERR,
			EK::Range => JSET::JSEXN_RANGEERR,
			EK::Reference => JSET::JSEXN_REFERENCEERR,
			EK::Syntax => JSET::JSEXN_SYNTAXERR,
			EK::Type => JSET::JSEXN_TYPEERR,
			EK::WasmCompile => JSET::JSEXN_WASMCOMPILEERROR,
			EK::WasmLink => JSET::JSEXN_WASMLINKERROR,
			EK::WasmRuntime => JSET::JSEXN_WASMRUNTIMEERROR,
			EK::None => JSET::JSEXN_ERR,
		}
	}
}

impl Display for ErrorKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		use ErrorKind as EK;
		let str = match self {
			EK::Normal => "Error",
			EK::Internal => "InternalError",
			EK::Aggregate => "AggregateError",
			EK::Eval => "EvalError",
			EK::Range => "RangeError",
			EK::Reference => "ReferenceError",
			EK::Syntax => "SyntaxError",
			EK::Type => "TypeError",
			EK::WasmCompile => "CompileError",
			EK::WasmLink => "LinkError",
			EK::WasmRuntime => "CompileError",
			EK::None => "Not an Error",
		};
		f.write_str(str)
	}
}

/// Represents errors in the JS Runtime
/// Contains information about the type of error, the error message and the error location.
///
/// If created from an error object, it also contains the error object.
#[derive(Clone, Debug)]
pub struct Error {
	pub kind: ErrorKind,
	pub message: String,
	pub location: Option<Location>,
	pub object: Option<*mut JSObject>,
}

impl Error {
	pub fn new<T: Into<Option<ErrorKind>>>(message: &str, kind: T) -> Error {
		Error {
			kind: kind.into().unwrap_or(ErrorKind::Normal),
			message: String::from(message),
			location: None,
			object: None,
		}
	}

	pub fn none() -> Error {
		Error {
			kind: ErrorKind::None,
			message: String::from(""),
			location: None,
			object: None,
		}
	}

	pub fn to_object<'cx>(&self, cx: &'cx Context) -> Option<Object<'cx>> {
		if let Some(object) = self.object {
			return Some(cx.root_object(object).into());
		}
		if self.kind != ErrorKind::None {
			unsafe {
				let exception_type = self.kind.to_exception_type();

				let stack = Stack::from_capture(cx).unwrap();
				let (file, lineno, column) = stack
					.records
					.first()
					.map(|record| &record.location)
					.map(|location| (&*location.file, location.lineno, location.column))
					.unwrap_or_default();

				let stack = Object::from(cx.root_object(stack.object.unwrap()));

				let file = file.as_value(cx);

				let file_name = cx.root_string(file.handle().to_string());

				let message = (!self.message.is_empty()).then(|| {
					let value = self.message.as_value(cx);
					crate::String::from(cx.root_string(value.handle().to_string()))
				});
				let message = message.unwrap_or_else(|| crate::String::from(cx.root_string(ptr::null_mut())));

				let mut error = Value::undefined(cx);

				if CreateError(
					cx.as_ptr(),
					exception_type,
					stack.handle().into(),
					file_name.handle().into(),
					lineno,
					column,
					ptr::null_mut(),
					message.handle().into(),
					UndefinedHandleValue,
					error.handle_mut().into(),
				) {
					return Some(error.to_object(cx));
				}
			}
		}
		None
	}

	pub fn format(&self) -> String {
		let Error { kind, message, location, .. } = self;
		let message = (!message.is_empty()).then(|| format!(" - {}", message)).unwrap_or(String::new());
		if let Some(location) = location {
			let Location { file, lineno, column } = location;
			if !file.is_empty() {
				return if *lineno == 0 {
					format!("{} at {}{}", kind, file, message)
				} else if *column == 0 {
					format!("{} at {}:{}{}", kind, file, lineno, message)
				} else {
					format!("{} at {}:{}:{}{}", kind, file, lineno, column, message)
				};
			}
		}
		format!("{}{}", kind, message)
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str(&self.message)
	}
}

impl<E: error::Error> From<E> for Error {
	fn from(error: E) -> Error {
		Error::new(&error.to_string(), None)
	}
}

impl ThrowException for Error {
	fn throw(&self, cx: &Context) {
		unsafe {
			use ErrorKind as EK;
			match self.kind {
				EK::Normal => JS_ReportErrorUTF8(cx.as_ptr(), format!("{}\0", self.message).as_ptr().cast()),
				EK::Internal => throw_internal_error(cx.as_ptr(), &self.message),
				EK::Range => throw_range_error(cx.as_ptr(), &self.message),
				EK::Type => throw_type_error(cx.as_ptr(), &self.message),
				EK::None => (),
				_ => unimplemented!("Throwing Exception for this is not implemented"),
			}
		}
	}
}

impl<'cx> ToValue<'cx> for Error {
	fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.to_object(cx).to_value(cx, value)
	}
}
