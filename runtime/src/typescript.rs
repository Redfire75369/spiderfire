/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::path::PathBuf;

use swc::common::{FileName, GLOBALS, Globals, Mark, SourceMap};
use swc::common::comments::{Comments, SingleThreadedComments};
use swc::common::errors::{ColorConfig, Handler};
use swc::common::input::StringInput;
use swc::common::sync::Lrc;
use swc_ecma_codegen::{Config as CodegenConfig, Emitter};
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_parser::{Capturing, Parser};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::Syntax;
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_base::hygiene::hygiene;
use swc_ecma_transforms_base::resolver;
use swc_ecma_transforms_typescript::strip;
use swc_ecma_visit::FoldWith;
use swc_ecmascript::ast::EsVersion;

use crate::config::Config;

pub fn compile_typescript(filename: &str, source: &str) -> String {
	if Config::global().typescript {
		if !Config::global().script {
			compile_typescript_module(filename, source)
		} else {
			compile_typescript_script(filename, source)
		}
	} else {
		String::from(source)
	}
}

pub fn compile_typescript_script(filename: &str, source: &str) -> String {
	let name = FileName::Real(PathBuf::from(filename));

	let source_map: Lrc<SourceMap> = Default::default();
	let file = source_map.new_source_file(name, String::from(source));
	let input = StringInput::from(&*file);

	let comments = SingleThreadedComments::default();
	let (handler, mut parser) = initialise_parser(source_map.clone(), &comments, input);

	let script = parser
		.parse_script()
		.map_err(|e| e.into_diagnostic(&handler).emit())
		.expect("Script parse failure");

	let globals = Globals::default();
	let script = GLOBALS.set(&globals, || {
		let unresolved_mark = Mark::new();
		let top_level_mark = Mark::new();

		let script = script.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));
		let script = script.fold_with(&mut strip(top_level_mark));
		let script = script.fold_with(&mut hygiene());
		script.fold_with(&mut fixer(Some(&comments)))
	});

	let mut buffer = Vec::new();
	let mut emitter = initialise_emitter(source_map.clone(), &comments, &mut buffer);
	emitter.emit_script(&script).expect("Script emission failure");

	String::from_utf8(buffer).expect("Emitted script contains invalid UTF-8")
}

pub fn compile_typescript_module(filename: &str, source: &str) -> String {
	let name = FileName::Real(PathBuf::from(filename));

	let source_map: Lrc<SourceMap> = Default::default();
	let file = source_map.new_source_file(name, String::from(source));
	let input = StringInput::from(&*file);

	let comments = SingleThreadedComments::default();
	let (handler, mut parser) = initialise_parser(source_map.clone(), &comments, input);

	let module = parser
		.parse_module()
		.map_err(|e| e.into_diagnostic(&handler).emit())
		.expect("Module parse failure");

	let globals = Globals::default();
	let module = GLOBALS.set(&globals, || {
		let unresolved_mark = Mark::new();
		let top_level_mark = Mark::new();

		let module = module.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));
		let module = module.fold_with(&mut strip(top_level_mark));
		let module = module.fold_with(&mut hygiene());
		module.fold_with(&mut fixer(Some(&comments)))
	});

	let mut buffer = Vec::new();
	let mut emitter = initialise_emitter(source_map.clone(), &comments, &mut buffer);
	emitter.emit_module(&module).expect("Module emission failure");

	String::from_utf8(buffer).expect("Emitted module contains invalid UTF-8")
}

fn initialise_parser<'a>(
	source_map: Lrc<SourceMap>, comments: &'a dyn Comments, input: StringInput<'a>,
) -> (Handler, Parser<Capturing<Lexer<'a, StringInput<'a>>>>) {
	let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(source_map.clone()));
	let lexer = Lexer::new(Syntax::Typescript(Default::default()), EsVersion::Es2022, input, Some(comments));
	let capturing = Capturing::new(lexer);
	let mut parser = Parser::new_from(capturing);

	for error in parser.take_errors() {
		error.into_diagnostic(&handler).emit();
	}

	(handler, parser)
}

fn initialise_emitter<'a>(
	source_map: Lrc<SourceMap>, comments: &'a dyn Comments, buffer: &'a mut Vec<u8>,
) -> Emitter<'a, JsWriter<'a, &'a mut Vec<u8>>, SourceMap> {
	Emitter {
		cfg: CodegenConfig {
			target: EsVersion::Es2022,
			minify: false,
			ascii_only: false,
		},
		cm: source_map.clone(),
		comments: Some(comments),
		wr: JsWriter::new(source_map, "\n", buffer, None),
	}
}
