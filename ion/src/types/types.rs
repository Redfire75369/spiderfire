use mozjs::jsapi::*;

type IonValue = *mut Value;

type IonUndefined = None;
type IonBoolean = bool;
type IonNumber = f64;
type IonString = *mut JSString;
type IonFunction = unsafe extern "C" fn(*mut JSContext, u32, IonValue) -> bool;
