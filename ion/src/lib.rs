/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate mozjs;

use std::result::Result as Result2;

use mozjs::jsapi::JSContext;

pub use class::ClassInitialiser;
pub use error::Error;
pub use exception::*;
pub use functions::*;
pub use ion_proc::*;
pub use objects::*;
pub use value::Value;

mod class;
mod error;
mod exception;
pub mod flags;
pub mod format;
mod functions;
mod objects;
pub mod spec;
pub mod types;
mod utils;
mod value;

pub type Context = *mut JSContext;
pub type Result<T> = Result2<T, Error>;
