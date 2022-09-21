/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::{Context, ErrorReport};
pub use loader::*;
pub use standard::*;

#[cfg(feature = "promise-logger")]
pub mod handler;
pub mod loader;
pub mod standard;

#[derive(Clone, Debug)]
pub struct ModuleError {
	pub kind: ModuleErrorKind,
	pub report: ErrorReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleErrorKind {
	Compilation,
	Instantiation,
	Evaluation,
}

impl ModuleError {
	fn new(report: ErrorReport, kind: ModuleErrorKind) -> ModuleError {
		ModuleError { kind, report }
	}

	pub fn format(&self, cx: Context) -> String {
		let str = match self.kind {
			ModuleErrorKind::Compilation => "Module Compilation Error",
			ModuleErrorKind::Instantiation => "Module Instantiation Error",
			ModuleErrorKind::Evaluation => "Module Evaluation Error",
		};
		format!("{}\n{}", str, self.report.format(cx))
	}
}
