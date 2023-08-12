/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate mozjs;

use std::result::Result as Result2;

pub use class::ClassInitialiser;
pub use context::Context;
pub use error::{Error, ErrorKind};
pub use exception::{ErrorReport, Exception, ThrowException};
pub use functions::{Arguments, Function};
#[cfg(feature = "macros")]
pub use ion_proc::*;
pub use local::Local;
pub use objects::{Array, Date, Object, OwnedKey, Promise, PropertyKey};
pub use objects::typedarray;
pub use stack::{Stack, StackRecord};
pub use string::String;
pub use symbol::Symbol;
pub use value::Value;

pub mod class;
mod context;
pub mod conversions;
mod error;
pub mod exception;
pub mod flags;
pub mod format;
pub mod functions;
pub mod local;
pub mod objects;
pub mod spec;
pub mod stack;
mod string;
pub mod symbol;
pub mod utils;
mod value;

pub type Result<T> = Result2<T, Error>;
pub type ResultExc<T> = Result2<T, Exception>;
