/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use indent::indent_by;

use crate::format::config::Config;
use crate::functions::function::IonFunction;
use crate::IonContext;

pub unsafe fn format_function(cx: IonContext, cfg: Config, function: IonFunction) -> String {
	indent_by((2 * (cfg.indentation + cfg.depth)) as usize, &function.to_string(cx))
}
