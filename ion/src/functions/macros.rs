/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::JSContext;

pub type IonContext = *mut JSContext;
pub type IonResult<T> = Result<T, Option<String>>;

#[macro_export]
macro_rules! js_fn_raw {
	(unsafe fn $name:ident($($param:ident : $type:ty), *) -> IonResult<$ret:ty> $body:tt) => {
		unsafe extern "C" fn $name(cx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
			use mozjs::conversions::ToJSValConvertible;

			let args = Arguments::new(argc, vp);

			unsafe fn native_fn($($param : $type),*) -> IonResult<$ret> $body

			match native_fn(cx, &args) {
			    Ok(v) => {
			        v.to_jsval(cx, mozjs::rust::MutableHandle::from_raw(args.rval()));
			        true
			    },
			    Err(Some(str)) => {
			        let cstr = CString::new(str).unwrap();
			        JS_ReportErrorUTF8(cx, cstr.as_ptr() as *const libc::c_char);
                    false
			    },
			    Err(None) => false
			}
		}
	}
}

#[macro_export]
macro_rules! js_fn_m {
    (fn $name:ident($($args:tt)*) -> IonResult<$ret:ty> $body:tt) => {
        #[apply(js_fn_raw!)]
		unsafe fn $name(cx: *mut JSContext, args: &Arguments) -> IonResult<$ret> {
			#[allow(unused_imports)]
			use mozjs::conversions::FromJSValConvertible;

            unpack_args!({stringify!($name), cx, args} ($($args)*));

            $body
        }
	}
}

#[macro_export]
macro_rules! unpack_args {
    ({$fn:expr, $cx:expr, $args:expr} ($($fn_args:tt)*)) => {
        if $args.len() < unpack_args_count!($($fn_args)*,) {
            return Err(Some(format!("{}() requires at least {} argument", $fn, unpack_args_count!($($fn_args)*,)).into()));
        }
        unpack_unwrap_args!(($cx, $args, 0) $($fn_args)*,);
    }
}

// [TODO]: Add support for mutable parameters
// [TODO]: Add support for optional parameters (Option<T>)
#[macro_export]
macro_rules! unpack_args_count {
    () => {0};
    (#[$special:ident] $name:ident: $type:ty, $($args:tt)*) => {
        unpack_args_count!($($args)*)
    };
    (#[$special:ident] $name:ident: $type:ty {$opt:expr}, $($args:tt)*) => {
        unpack_args_count!($($args)*)
    };
	($name:ident: IonContext, $($args:tt)*) => {
        unpack_args_count!($($args)*)
    };
	($name:ident: $ty:ty, $($args:tt)*) => {
        1 + unpack_args_count!($($args)*)
    };
    ($name:ident: $ty:ty {$opt:expr}, $($args:tt)*) => {
        1 + unpack_args_count!($($args)*)
    };
    (, $($rest:tt)*) => {
        unpack_args_count!($($rest)*)
    };
}

#[macro_export]
macro_rules! unpack_unwrap_args {
    (($cx:expr, $args:expr, $n:expr) $(,)*) => {};
    // Special Case: #[this]
    (($cx:expr, $args:expr, $n:expr) #[this] $name:ident : $type:ty, $($fn_args:tt)*) => {
        let $name = <*mut JSObject as FromJSValConvertible>::from_jsval($cx, mozjs::rust::Handle::from_raw($args.thisv()), ()).unwrap().get_success_value().unwrap().clone();
        unpack_unwrap_args!(($cx, $args, $n) $($fn_args)*);
    };
    // Special Case: #[this] with Conversion Options
    (($cx:expr, $args:expr, $n:expr) #[this] $name:ident : $type:ty {$opt:expr}, $($fn_args:tt)*) => {
		let $name = <$type as FromJSValConvertible>::from_jsval($cx, mozjs::rust::Handle::from_raw($args.thisv()), $opt).unwrap().get_success_value().unwrap().clone();
        unpack_unwrap_args!(($cx, $args, $n) $($fn_args)*);
    };
	// Special Case: Variable Args #[varargs]
    (($cx:expr, $args:expr, $n:expr) #[varargs] $name:ident : Vec<$type:ty>, ) => {
		let $name = $args.range_handles($n..($args.len() + 1)).iter().map::<::std::result::Result<$type, ()>, _>(|arg| {
            Ok(<$type as FromJSValConvertible>::from_jsval($cx, mozjs::rust::Handle::from_raw(arg.clone()), ())?.get_success_value().unwrap().clone())
        }).collect::<::std::result::Result<Vec<$type>, _>>().unwrap();
    };
	// Special Case: Variable Args #[varargs] with Conversion Options
    (($cx:expr, $args:expr, $n:expr) #[varargs] $name:ident : Vec<$type:ty> {$opt:expr}, ) => {
		let $name = $args.range_handles($n..($args.len() + 1)).iter().map(|arg| {
            Ok(<$type as FromJSValConvertible>::from_jsval($cx, mozjs::rust::Handle::from_raw(arg.clone()), $opt)?.get_success_value().unwrap().clone())
        }).collect::<::std::result::Result<Vec<$type>, _>>().unwrap();
    };
	// Special Case: IonContext
    (($cx:expr, $args:expr, $n:expr) $name:ident : IonContext, $($fn_args:tt)*) => {
		let $name = $cx;
		unpack_unwrap_args!(($cx, $args, $n) $($fn_args)*);
    };
	// No Conversion Options
    (($cx:expr, $args:expr, $n:expr) $name:ident : $type:ty, $($fn_args:tt)*) => {
        let $name = <$type as FromJSValConvertible>::from_jsval($cx, mozjs::rust::Handle::from_raw($args.handle($n).unwrap()), ()).unwrap().get_success_value().unwrap().clone();
        unpack_unwrap_args!(($cx, $args, $n+1) $($fn_args)*);
    };
    // With Conversion Options
    (($cx:expr, $args:expr, $n:expr) $name:ident : $type:ty {$opt:expr}, $($fn_args:tt)*) => {
        let $name = <$type as FromJSValConvertible>::from_jsval($cx, mozjs::rust::Handle::from_raw($args.handle($n).unwrap()), $opt).unwrap().get_success_value().unwrap().clone();
        unpack_unwrap_args!(($cx, $args, $n+1) $($fn_args)*);
    };
}
