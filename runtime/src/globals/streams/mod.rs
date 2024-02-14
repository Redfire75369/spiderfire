/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ion::{ClassDefinition, Context, Object};

use crate::globals::streams::readable::{
	ByobReader, ByteStreamController, DefaultController, DefaultReader, ReadableStream,
};

mod readable;

pub fn define<'cx>(cx: &'cx Context, global: &'cx Object) -> bool {
	ReadableStream::init_class(cx, global).0
		&& ByteStreamController::init_class(cx, global).0
		&& DefaultController::init_class(cx, global).0
		&& DefaultReader::init_class(cx, global).0
		&& ByobReader::init_class(cx, global).0
}
