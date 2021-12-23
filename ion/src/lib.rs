/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_use]
extern crate mozjs;

use mozjs::jsapi::JSContext;

pub use ion_proc::*;

use crate::error::IonError;

pub mod error;
pub mod exception;
pub mod format;
pub mod functions;
pub mod objects;
pub mod script;
pub mod spec;
pub mod types;

pub type IonContext = *mut JSContext;
pub type IonResult<T> = Result<T, IonError>;
