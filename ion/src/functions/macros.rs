/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_export(local_inner_macros)]
macro_rules! js_fn_raw_m {
	($pub:vis unsafe fn $name:ident($($param:ident : $type:ty),*) -> IonResult<$ret:ty> $body:tt) => {
		#[allow(non_snake_case)]
		$pub unsafe extern "C" fn $name(cx: $crate::IonContext, argc: u32, vp: *mut ::mozjs::jsapi::Value) -> bool {
			#[allow(unused_import)]
			use ::mozjs::conversions::ToJSValConvertible;

			let args = $crate::functions::arguments::Arguments::new(argc, vp);

			unsafe fn native_fn($($param : $type),*) -> $crate::IonResult<$ret> $body

			match native_fn(cx, &args) {
				Ok(v) => {
					v.to_jsval(cx, ::mozjs::rust::MutableHandle::from_raw(args.rval()));
					true
				},
				Err(error) => {
					error.throw(cx);
					false
				}
			}
		}
	};
}

#[macro_export(local_inner_macros)]
macro_rules! js_fn_m {
	($pub:vis unsafe fn $name:ident($($args:tt)*) -> IonResult<$ret:ty> $body:tt) => {
		js_fn_raw_m!{
			$pub unsafe fn $name(cx: IonContext, args: &Arguments) -> IonResult<$ret> {
				#[allow(unused_imports)]
				use ::mozjs::conversions::FromJSValConvertible;

				unpack_args!((::std::stringify!($name), cx, args) ($($args)*));

				$body
			}
		}
	};
	($pub:vis async unsafe fn $name:ident($($args:tt)*) -> Result<$res:ty, $rej:ty> $body:tt) => {
		js_fn_raw_m! {
			$pub unsafe fn $name(cx: IonContext, args: &Arguments) -> IonResult<$crate::objects::promise::IonPromise> {
				#[allow(unused_imports)]
				use ::mozjs::conversions::FromJSValConvertible;

				unpack_args!((::std::stringify!($name), cx, args) ($($args)*));

				let future = async $body;

				if let Some(promise) = $crate::objects::promise::IonPromise::new_with_future(cx, future) {
					Ok(promise)
				} else {
					Err($crate::error::IonError::None)
				}
			}
		}
	};
}

#[macro_export(local_inner_macros)]
macro_rules! unpack_args {
	(($fn:expr, $cx:expr, $args:expr) ($($fn_args:tt)*)) => {
		let nargs = unpack_args_count!($($fn_args)*,);
		if $args.len() < nargs {
			return Err($crate::error::IonError::Error(::std::format!("{}() requires at least {} argument", $fn, nargs).into()));
		}
		unpack_unwrap_args!(($cx, $args, 0) $($fn_args)*,);
	}
}

#[macro_export(local_inner_macros)]
macro_rules! unpack_args_count {
	() => {0};
	($name:ident: IonContext, $($args:tt)*) => {
		unpack_args_count!($($args)*)
	};
	($(#[$special:meta])? $(mut)? $name:ident : Option<$type:ty>, $($args:tt)*) => {
		1
	};
	($(#[$special:meta])? $(mut)? $name:ident : $type:ty, $($args:tt)*) => {
		1 + unpack_args_count!($($args)*)
	};
	(, $($rest:tt)*) => {
		unpack_args_count!($($rest)*)
	};
}

#[macro_export(local_inner_macros)]
macro_rules! unpack_unwrap_args {
	(($cx:expr, $args:expr, $n:expr) $(,)*) => {};
	// Special Case: #[this]
	(($cx:expr, $args:expr, $n:expr) #[this] $name:ident : $type:ty, $($fn_args:tt)*) => {
		let $name = $crate::objects::object::IonObject::from_value($cx, $args.this());
		unpack_unwrap_args!(($cx, $args, $n) $($fn_args)*);
	};
	// Special Case: Variable Args #[varargs]
	(($cx:expr, $args:expr, $n:expr) #[varargs] $name:ident : Vec<$type:ty>, ) => {
		let $name: Vec<$type> = $args.range_handles($n..($args.len() + 1)).iter().enumerate().map(|(index, handle)| {
			unwrap_arg!(($cx, $n+index) $name: $type, handle)
		}).collect::<IonResult<_>>()?;
	};
	// Special Case: Mutable Variable Args #[varargs]
	(($cx:expr, $args:expr, $n:expr) #[varargs] mut $name:ident : Vec<$type:ty>, ) => {
		let mut $name: Vec<$type> = $args.range_handles($n..($args.len() + 1)).iter().enumerate().map(|(index, handle)| {
			unwrap_arg!(($cx, $n+index) $name: $type, handle)
		}).collect::<IonResult<_>>()?;
	};
	// Special Case: IonContext
	(($cx:expr, $args:expr, $n:expr) $name:ident : IonContext, $($fn_args:tt)*) => {
		let $name = $cx;
		unpack_unwrap_args!(($cx, $args, $n) $($fn_args)*);
	};
	// Special Case: Arguments
	(($cx:expr, $args:expr, $n:expr) $name:ident : Arguments, $($fn_args:tt)*) => {
		let $name = $args;
		unpack_unwrap_args!(($cx, $args, $n) $($fn_args)*);
	};
	// Default Case
	(($cx:expr, $args:expr, $n:expr) $name:ident : $type:ty, $($fn_args:tt)*) => {
		let $name: $type  = unwrap_arg!(($cx, $n) $name: $type, $args.handle_or_undefined($n))?;
		unpack_unwrap_args!(($cx, $args, $n+1) $($fn_args)*);
	};
	// Default Mutable Case
	(($cx:expr, $args:expr, $n:expr) mut $name:ident : $type:ty, $($fn_args:tt)*) => {
		let mut $name: $type = unwrap_arg!(($cx, $n) $name: $type, $args.handle_or_undefined($n))?;
		unpack_unwrap_args!(($cx, $args, $n+1) $($fn_args)*);
	};
}

#[macro_export(local_inner_macros)]
macro_rules! unwrap_arg {
	(($cx:expr, $n:expr) $name:ident : $type:ty, $handle:expr) => {
		if let Some(value) = $crate::types::values::from_value($cx, $handle.get(), ()) {
			Ok(value)
		} else {
			Err($crate::error::IonError::TypeError(::std::format!(
				"Failed to convert argument {} at index {}, to {}",
				::std::stringify!($name),
				$n,
				::std::stringify!($type)
			)))
		}
	};
}
