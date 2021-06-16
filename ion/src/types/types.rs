/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::*;

type IonValue = *mut Value;

type IonBoolean = bool;
type IonNumber = f64;
type IonString = *mut JSString;
type IonFunction = unsafe extern "C" fn(*mut JSContext, u32, IonValue) -> bool;
