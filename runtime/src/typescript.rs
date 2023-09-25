/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::string::FromUtf8Error;

use sourcemap::SourceMap;
use swc_core::common::{BytePos, FileName, Globals, GLOBALS, LineCol, Mark, SourceMap as SwcSourceMap};
use swc_core::common::comments::{Comments, SingleThreadedComments};
use swc_core::common::errors::{ColorConfig, Handler};
use swc_core::common::input::StringInput;
use swc_core::common::sync::Lrc;
use swc_ecmascript::ast::EsVersion;
use swc_ecmascript::codegen::{Config as CodegenConfig, Emitter};
use swc_ecmascript::codegen::text_writer::JsWriter;
use swc_ecmascript::parser::{Capturing, Parser, Syntax};
use swc_ecmascript::parser::lexer::Lexer;
use swc_ecmascript::transforms::{fixer, hygiene, resolver};
use swc_ecmascript::transforms::typescript::strip;
use swc_ecmascript::visit::FoldWith;

use crate::config::Config;

pub fn compile_typescript(filename: &str, source: &str) -> Result<(String, SourceMap), Error> {
	let name = FileName::Real(PathBuf::from(filename));

	let source_map: Lrc<SwcSourceMap> = Default::default();
	let file = source_map.new_source_file(name, String::from(source));
	let input = StringInput::from(&*file);

	let comments = SingleThreadedComments::default();
	let (handler, mut parser) = initialise_parser(source_map.clone(), &comments, input);

	let mut buffer = Vec::new();
	let mut mappings = Vec::new();
	let mut emitter = initialise_emitter(source_map.clone(), &comments, &mut buffer, &mut mappings);

	if Config::global().script {
		handle_script(&handler, &mut parser, &mut emitter)?;
	} else {
		handle_module(&handler, &mut parser, &mut emitter)?;
	}

	let source_map = source_map.build_source_map(&mappings);
	Ok((String::from_utf8(buffer)?, source_map))
}

pub fn handle_script(
	handler: &Handler, parser: &mut Parser<Capturing<Lexer>>, emitter: &mut Emitter<JsWriter<&mut Vec<u8>>, SwcSourceMap>,
) -> Result<(), Error> {
	let script = parser.parse_script().map_err(|e| {
		e.into_diagnostic(handler).emit();
		Error::Parse
	})?;

	let comments = emitter.comments;
	let globals = Globals::default();
	let script = GLOBALS.set(&globals, || {
		let unresolved_mark = Mark::new();
		let top_level_mark = Mark::new();

		let script = script.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));
		let script = script.fold_with(&mut strip(top_level_mark));
		let script = script.fold_with(&mut hygiene());
		script.fold_with(&mut fixer(comments))
	});

	emitter.emit_script(&script).map_err(|_| Error::Emission)
}

pub fn handle_module(
	handler: &Handler, parser: &mut Parser<Capturing<Lexer>>, emitter: &mut Emitter<JsWriter<&mut Vec<u8>>, SwcSourceMap>,
) -> Result<(), Error> {
	let module = parser.parse_module().map_err(|e| {
		e.into_diagnostic(handler).emit();
		Error::Parse
	})?;

	let comments = emitter.comments;
	let globals = Globals::default();
	let module = GLOBALS.set(&globals, || {
		let unresolved_mark = Mark::new();
		let top_level_mark = Mark::new();

		let module = module.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));
		let module = module.fold_with(&mut strip(top_level_mark));
		let module = module.fold_with(&mut hygiene());
		module.fold_with(&mut fixer(comments))
	});

	emitter.emit_module(&module).map_err(|_| Error::Emission)
}

fn initialise_parser<'a>(
	source_map: Lrc<SwcSourceMap>, comments: &'a dyn Comments, input: StringInput<'a>,
) -> (Handler, Parser<Capturing<Lexer<'a>>>) {
	let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(source_map));
	let lexer = Lexer::new(Syntax::Typescript(Default::default()), EsVersion::Es2022, input, Some(comments));
	let capturing = Capturing::new(lexer);
	let mut parser = Parser::new_from(capturing);

	for error in parser.take_errors() {
		error.into_diagnostic(&handler).emit();
	}

	(handler, parser)
}

fn initialise_emitter<'a>(
	source_map: Lrc<SwcSourceMap>, comments: &'a dyn Comments, buffer: &'a mut Vec<u8>, mappings: &'a mut Vec<(BytePos, LineCol)>,
) -> Emitter<'a, JsWriter<'a, &'a mut Vec<u8>>, SwcSourceMap> {
	Emitter {
		cfg: CodegenConfig {
			target: EsVersion::Es2022,
			ascii_only: false,
			minify: false,
			omit_last_semi: false,
		},
		cm: source_map.clone(),
		comments: Some(comments),
		wr: JsWriter::new(source_map, "\n", buffer, Some(mappings)),
	}
}

#[derive(Debug)]
pub enum Error {
	Parse,
	Emission,
	FromUtf8(FromUtf8Error),
}

impl From<FromUtf8Error> for Error {
	fn from(err: FromUtf8Error) -> Error {
		Error::FromUtf8(err)
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Error::FromUtf8(err) => f.write_str(&err.to_string()),
			_ => Ok(()),
		}
	}
}
