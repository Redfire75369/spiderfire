/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{error, ptr};
use std::fmt::{Display, Formatter};

use mozjs::error::{throw_internal_error, throw_range_error, throw_type_error};
use mozjs::jsapi::{CreateError, JS_ReportErrorUTF8, JSExnType, JSObject, JSProtoKey, JSString};
use mozjs::jsval::UndefinedValue;

use crate::{Context, Location, Object, Stack, Value};
use crate::conversions::ToValue;

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
	Compile,
	Link,
	Runtime,
	None,
}

/// Represents errors that can be thrown in the runtime.
#[derive(Clone, Debug)]
pub struct Error {
	pub kind: ErrorKind,
	pub message: String,
	pub location: Option<Location>,
	pub object: Option<*mut JSObject>,
}

pub trait ThrowException {
	fn throw(&self, cx: &Context);
}

impl ErrorKind {
	pub fn from_proto_key(key: JSProtoKey) -> ErrorKind {
		use JSProtoKey::{
			JSProto_AggregateError, JSProto_CompileError, JSProto_Error, JSProto_EvalError, JSProto_InternalError, JSProto_LinkError,
			JSProto_RangeError, JSProto_ReferenceError, JSProto_RuntimeError, JSProto_SyntaxError, JSProto_TypeError,
		};
		use ErrorKind::*;
		match key {
			JSProto_Error => Normal,
			JSProto_InternalError => Internal,
			JSProto_AggregateError => Aggregate,
			JSProto_EvalError => Eval,
			JSProto_RangeError => Range,
			JSProto_ReferenceError => Reference,
			JSProto_SyntaxError => Syntax,
			JSProto_TypeError => Type,
			JSProto_CompileError => Compile,
			JSProto_LinkError => Link,
			JSProto_RuntimeError => Runtime,
			_ => None,
		}
	}

	pub fn to_exception_type(&self) -> JSExnType {
		use ErrorKind::*;
		use JSExnType::*;
		match self {
			Normal => JSEXN_ERR,
			Internal => JSEXN_INTERNALERR,
			Aggregate => JSEXN_AGGREGATEERR,
			Eval => JSEXN_EVALERR,
			Range => JSEXN_RANGEERR,
			Reference => JSEXN_REFERENCEERR,
			Syntax => JSEXN_SYNTAXERR,
			Type => JSEXN_TYPEERR,
			Compile => JSEXN_WASMCOMPILEERROR,
			Link => JSEXN_WASMLINKERROR,
			Runtime => JSEXN_WASMRUNTIMEERROR,
			None => JSEXN_ERR,
		}
	}
}

impl Display for ErrorKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		use ErrorKind::*;
		let str = match self {
			Normal => "Error",
			Internal => "InternalError",
			Aggregate => "AggregateError",
			Eval => "EvalError",
			Range => "RangeError",
			Reference => "ReferenceError",
			Syntax => "SyntaxError",
			Type => "TypeError",
			Compile => "CompileError",
			Link => "LinkError",
			Runtime => "CompileError",
			None => "Not an Error",
		};
		f.write_str(str)
	}
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

				let mut file_name = Value::undefined(cx);
				file.to_value(cx, &mut file_name);

				let file_name = cx.root_string(file_name.handle().get().to_string());

				rooted!(in(**cx) let mut message: *mut JSString);
				if !self.message.is_empty() {
					let mut message_val = Value::undefined(cx);
					self.message.to_value(cx, &mut message_val);
					message.set(message_val.handle().get().to_string());
				}

				let mut error = Value::from(cx.root_value(UndefinedValue()));

				if CreateError(
					**cx,
					exception_type,
					stack.handle().into(),
					file_name.handle().into(),
					lineno,
					column,
					ptr::null_mut(),
					message.handle().into(),
					error.handle_mut().into(),
				) {
					return Some(error.to_object(cx));
				}
			}
		}
		None
	}
}

impl ThrowException for Error {
	fn throw(&self, cx: &Context) {
		unsafe {
			use ErrorKind::*;
			match self.kind {
				Normal => JS_ReportErrorUTF8(**cx, format!("{}\0", self.message).as_ptr() as *const i8),
				Internal => throw_internal_error(**cx, &self.message),
				Range => throw_range_error(**cx, &self.message),
				Type => throw_type_error(**cx, &self.message),
				None => (),
				_ => unimplemented!("Throwing Exception for this is not implemented"),
			}
		}
	}
}

impl<'cx> ToValue<'cx> for Error {
	unsafe fn to_value(&self, cx: &'cx Context, value: &mut Value) {
		self.to_object(cx).to_value(cx, value)
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}

impl<E: error::Error> From<E> for Error {
	fn from(error: E) -> Error {
		Error::new(&error.to_string(), None)
	}
}
