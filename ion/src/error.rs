/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{error, ptr};
use std::fmt::{Display, Formatter};

use mozjs::conversions::ToJSValConvertible;
use mozjs::error::{throw_internal_error, throw_range_error, throw_type_error};
use mozjs::jsapi::{CreateError, JS_ReportErrorUTF8, JSExnType, JSProtoKey, JSString};
use mozjs::jsval::JSVal;
use mozjs::rust::MutableHandleValue;

use crate::{Context, Location, Object, Stack};
use crate::conversions::IntoJSVal;

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
	pub object: Option<Object>,
}

pub trait ThrowException {
	fn throw(&self, cx: Context);
}

impl ErrorKind {
	pub fn from_proto_key(key: JSProtoKey) -> ErrorKind {
		use JSProtoKey::{
			JSProto_Error, JSProto_InternalError, JSProto_AggregateError, JSProto_EvalError, JSProto_RangeError, JSProto_ReferenceError,
			JSProto_SyntaxError, JSProto_TypeError, JSProto_CompileError, JSProto_LinkError, JSProto_RuntimeError,
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
	pub fn new(message: &str, kind: Option<ErrorKind>) -> Error {
		Error {
			kind: kind.unwrap_or(ErrorKind::Normal),
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

	pub fn to_object(&self, cx: Context) -> Option<Object> {
		if let Some(object) = self.object {
			return Some(object);
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

				rooted!(in(cx) let stack = *stack.object.unwrap());

				rooted!(in(cx) let mut file_name: JSVal);
				file.to_jsval(cx, file_name.handle_mut());
				rooted!(in(cx) let file_name = file_name.get().to_string());

				rooted!(in(cx) let mut message: *mut JSString);
				if !self.message.is_empty() {
					rooted!(in(cx) let mut message_val: JSVal);
					self.message.to_jsval(cx, message_val.handle_mut());
					message.set(message_val.get().to_string());
				}

				rooted!(in(cx) let mut error: JSVal);

				if CreateError(
					cx,
					exception_type,
					stack.handle().into(),
					file_name.handle().into(),
					lineno,
					column,
					ptr::null_mut(),
					message.handle().into(),
					error.handle_mut().into(),
				) {
					return Some(Object::from(error.get().to_object()));
				}
			}
		}
		None
	}
}

impl ThrowException for Error {
	fn throw(&self, cx: Context) {
		unsafe {
			use ErrorKind::*;
			match self.kind {
				Normal => JS_ReportErrorUTF8(cx, format!("{}\0", self.message).as_ptr() as *const i8),
				Internal => throw_internal_error(cx, &self.message),
				Range => throw_range_error(cx, &self.message),
				Type => throw_type_error(cx, &self.message),
				None => (),
				_ => unimplemented!("Throwing Exception for this is not implemented"),
			}
		}
	}
}

impl IntoJSVal for Error {
	unsafe fn into_jsval(self: Box<Self>, cx: Context, mut rval: MutableHandleValue) {
		if let Some(object) = self.to_object(cx) {
			rval.set(object.to_value());
		}
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
