/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};

use ion::ErrorReport;
pub use loader::*;
pub use standard::*;

pub mod loader;
pub mod standard;

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleError {
	pub kind: ModuleErrorKind,
	pub report: ErrorReport,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModuleErrorKind {
	Compilation,
	Instantiation,
	Evaluation,
}

impl ModuleError {
	fn new(report: ErrorReport, kind: ModuleErrorKind) -> ModuleError {
		ModuleError { kind, report }
	}
}

impl Display for ModuleError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self.kind {
			ModuleErrorKind::Compilation => f.write_str("Module Compilation Error\n{}")?,
			ModuleErrorKind::Instantiation => f.write_str("Module Instantiation Error\n{}")?,
			ModuleErrorKind::Evaluation => f.write_str("Module Evaluation Error\n{}")?,
		}
		f.write_str(&self.report.to_string())
	}
}
