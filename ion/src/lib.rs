/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![allow(clippy::not_unsafe_ptr_arg_deref)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate mozjs;

use std::result::Result as Result2;

use mozjs::jsapi::JSContext;

pub use class::ClassInitialiser;
pub use error::{Error, ErrorKind};
pub use exception::*;
pub use functions::*;
#[cfg(feature = "macros")]
pub use ion_proc::*;
pub use objects::*;
pub use stack::*;
pub use value::Value;

mod class;
pub mod conversions;
pub mod error;
mod exception;
pub mod flags;
pub mod format;
mod functions;
mod objects;
pub mod spec;
mod stack;
pub mod typedarray;
pub mod types;
pub mod utils;
mod value;

pub type Context = *mut JSContext;
pub type Result<T> = Result2<T, Error>;
