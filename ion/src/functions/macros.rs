/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::JSContext;

pub type IonContext = *mut JSContext;
pub type IonResult<T> = Result<T, Option<String>>;

#[macro_export]
macro_rules! js_fn_raw_m {
	(unsafe fn $name:ident($($param:ident : $type:ty), *) -> IonResult<$ret:ty> $body:tt) => {
		#[allow(non_snake_case)]
		unsafe extern "C" fn $name(cx: IonContext, argc: u32, vp: *mut Value) -> bool {
			use ::mozjs::conversions::ToJSValConvertible;

			let args = Arguments::new(argc, vp);

			unsafe fn native_fn($($param : $type),*) -> IonResult<$ret> $body

			match native_fn(cx, &args) {
				Ok(v) => {
					v.to_jsval(cx, ::mozjs::rust::MutableHandle::from_raw(args.rval));
					true
				},
				Err(Some(str)) => {
					let cstr = ::std::ffi::CString::new(str).unwrap();
					::mozjs::jsapi::JS_ReportErrorUTF8(cx, cstr.as_ptr() as *const i8);
					false
				},
				Err(None) => false
			}
		}
	}
}

#[macro_export]
macro_rules! js_fn_m {
	(unsafe fn $name:ident($($args:tt)*) -> IonResult<$ret:ty> $body:tt) => {
		js_fn_raw_m!(unsafe fn $name(cx: IonContext, args: &Arguments) -> IonResult<$ret> {
			#[allow(unused_imports)]
			use ::mozjs::conversions::FromJSValConvertible;

			unpack_args!({stringify!($name), cx, args} ($($args)*));

			$body
		});
	}
}

#[macro_export]
macro_rules! unpack_args {
	({$fn:expr, $cx:expr, $args:expr} ($($fn_args:tt)*)) => {
		let nargs = unpack_args_count!($($fn_args)*,);
		if $args.len() < nargs {
			return Err(Some(format!("{}() requires at least {} argument", $fn, nargs).into()));
		}
		unpack_unwrap_args!(($cx, $args, 0) $($fn_args)*,);
	}
}

#[macro_export]
macro_rules! unpack_args_count {
	() => {0};
	($name:ident: IonContext, $($args:tt)*) => {
		unpack_args_count!($($args)*)
	};
	($(#[$special:ident])? $(mut)? $name:ident: Option<$type:ty>, $($args:tt)*) => {
		1
	};
	($(#[$special:ident])? $(mut)? $name:ident: $type:ty, $($args:tt)*) => {
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
		let $name = <IonObject as FromJSValConvertible>::from_jsval($cx, ::mozjs::rust::Handle::from_raw($args.this), ()).unwrap().get_success_value().unwrap().clone();
		unpack_unwrap_args!(($cx, $args, $n) $($fn_args)*);
	};
	// Special Case: Variable Args #[varargs]
	(($cx:expr, $args:expr, $n:expr) #[varargs] $name:ident : Vec<$type:ty>, ) => {
		let $name = $args.range_handles($n..($args.len() + 1)).iter().map::<::std::result::Result<$type, ()>, _>(|arg| {
			Ok(<$type as FromJSValConvertible>::from_jsval($cx, ::mozjs::rust::Handle::from_raw(arg.clone()), ())?.get_success_value().unwrap().clone())
		}).collect::<::std::result::Result<Vec<$type>, _>>().unwrap();
	};
	// Special Case: Mutable Variable Args #[varargs]
	(($cx:expr, $args:expr, $n:expr) #[varargs] mut $name:ident : Vec<$type:ty>, ) => {
		let mut $name = $args.range_handles($n..($args.len() + 1)).iter().map::<::std::result::Result<$type, ()>, _>(|arg| {
			Ok(<$type as FromJSValConvertible>::from_jsval($cx, ::mozjs::rust::Handle::from_raw(arg.clone()), ())?.get_success_value().unwrap().clone())
		}).collect::<::std::result::Result<Vec<$type>, _>>().unwrap();
	};
	// Special Case: IonContext
	(($cx:expr, $args:expr, $n:expr) $name:ident : IonContext, $($fn_args:tt)*) => {
		let $name = $cx;
		unpack_unwrap_args!(($cx, $args, $n) $($fn_args)*);
	};
	// Default Case
	(($cx:expr, $args:expr, $n:expr) $name:ident : $type:ty, $($fn_args:tt)*) => {
		let $name = <$type as FromJSValConvertible>::from_jsval($cx, ::mozjs::rust::Handle::from_raw($args.handle_or_undefined($n)), ()).unwrap().get_success_value().unwrap().clone();
		unpack_unwrap_args!(($cx, $args, $n+1) $($fn_args)*);
	};
	// Default Mutable Case
	(($cx:expr, $args:expr, $n:expr) mut $name:ident : $type:ty, $($fn_args:tt)*) => {
		let mut $name = <$type as FromJSValConvertible>::from_jsval($cx, ::mozjs::rust::Handle::from_raw($args.handle_or_undefined($n)), ()).unwrap().get_success_value().unwrap().clone();
		unpack_unwrap_args!(($cx, $args, $n+1) $($fn_args)*);
	};
}
