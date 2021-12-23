/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use mozjs::jsapi::{
	JSNativeWrapper, JSPROP_ENUMERATE, JSPROP_PERMANENT, JSPROP_READONLY, JSPropertySpec, JSPropertySpec_Accessor, JSPropertySpec_AccessorsOrValue,
	JSPropertySpec_AccessorsOrValue_Accessors, JSPropertySpec_Name, JSPropertySpec_ValueWrapper, JSPropertySpec_ValueWrapper__bindgen_ty_1,
	JSPropertySpec_ValueWrapper_Type,
};

pub const JSPROP_CONSTANT: u16 = (JSPROP_READONLY | JSPROP_PERMANENT) as u16;
pub const JSPROP_CONSTANT_ENUMERATED: u16 = (JSPROP_READONLY | JSPROP_ENUMERATE | JSPROP_PERMANENT) as u16;

pub const fn create_property_spec_accessor(name: &'static str, getter: JSNativeWrapper, setter: JSNativeWrapper, attrs: u8) -> JSPropertySpec {
	JSPropertySpec {
		name: JSPropertySpec_Name { string_: name.as_ptr() as *const i8 },
		attributes_: attrs,
		isAccessor_: true,
		u: JSPropertySpec_AccessorsOrValue {
			accessors: JSPropertySpec_AccessorsOrValue_Accessors {
				getter: JSPropertySpec_Accessor { native: getter },
				setter: JSPropertySpec_Accessor { native: setter },
			},
		},
	}
}

pub const fn create_property_spec_string(name: &'static str, string: &'static str, attrs: u8) -> JSPropertySpec {
	JSPropertySpec {
		name: JSPropertySpec_Name { string_: name.as_ptr() as *const i8 },
		attributes_: attrs,
		isAccessor_: false,
		u: JSPropertySpec_AccessorsOrValue {
			value: JSPropertySpec_ValueWrapper {
				type_: JSPropertySpec_ValueWrapper_Type::String,
				__bindgen_anon_1: JSPropertySpec_ValueWrapper__bindgen_ty_1 { string: string as *const _ as *const i8 },
			},
		},
	}
}

pub const fn create_property_spec_int(name: &'static str, int: i32, attrs: u8) -> JSPropertySpec {
	JSPropertySpec {
		name: JSPropertySpec_Name { string_: name.as_ptr() as *const i8 },
		attributes_: attrs,
		isAccessor_: false,
		u: JSPropertySpec_AccessorsOrValue {
			value: JSPropertySpec_ValueWrapper {
				type_: JSPropertySpec_ValueWrapper_Type::Int32,
				__bindgen_anon_1: JSPropertySpec_ValueWrapper__bindgen_ty_1 { int32: int },
			},
		},
	}
}

pub const fn create_property_spec_double(name: &'static str, double: f64, attrs: u8) -> JSPropertySpec {
	JSPropertySpec {
		name: JSPropertySpec_Name { string_: name.as_ptr() as *const i8 },
		attributes_: attrs,
		isAccessor_: false,
		u: JSPropertySpec_AccessorsOrValue {
			value: JSPropertySpec_ValueWrapper {
				type_: JSPropertySpec_ValueWrapper_Type::Double,
				__bindgen_anon_1: JSPropertySpec_ValueWrapper__bindgen_ty_1 { double_: double },
			},
		},
	}
}

#[macro_export(local_inner_macros)]
macro_rules! property_spec_getter {
	($getter:expr) => {
		property_spec_getter!($getter, ::std::stringify!($getter))
	};
	($getter:expr, $name:expr) => {
		property_spec_getter!($getter, $name, ::mozjs::jsapi::JSPROP_ENUMERATE)
	};
	($getter:expr, $name:expr, $attrs:expr) => {
		$crate::spec::create_property_spec_accessor(
			::std::concat!($name, "\0"),
			::mozjs::jsapi::JSNativeWrapper {
				op: Some($getter),
				info: ::std::ptr::null_mut(),
			},
			::mozjs::jsapi::JSNativeWrapper { op: None, info: ::std::ptr::null_mut() },
			$attrs,
		)
	};
}

#[macro_export(local_inner_macros)]
macro_rules! property_spec_setter {
	($setter:expr) => {
		property_spec_setter!($setter, ::std::stringify!($setter))
	};
	($setter:expr, $name:expr) => {
		property_spec_setter!($setter, $name, ::mozjs::jsapi::JSPROP_ENUMERATE)
	};
	($setter:expr, $name:expr, $attrs:expr) => {
		$crate::spec::create_property_spec_accessor(
			::std::concat!($name, "\0"),
			::mozjs::jsapi::JSNativeWrapper { op: None, info: ::std::ptr::null_mut() },
			::mozjs::jsapi::JSNativeWrapper {
				op: Some($setter),
				info: ::std::ptr::null_mut(),
			},
			$attrs,
		)
	};
}

#[macro_export(local_inner_macros)]
macro_rules! property_spec_getter_setter {
	($getter:expr, $setter:expr, $name:expr, $attrs:expr) => {
		$crate::spec::create_property_spec_accessor(
			::std::concat!($name, "\0"),
			::mozjs::jsapi::JSNativeWrapper {
				op: Some($getter),
				info: ::std::ptr::null_mut(),
			},
			::mozjs::jsapi::JSNativeWrapper {
				op: Some($setter),
				info: ::std::ptr::null_mut(),
			},
			$attrs,
		)
	};
	($getter:expr, $setter:expr, $name:expr) => {
		property_spec_getter_setter!($getter, $setter, $name, ::mozjs::jsapi::JSPROP_ENUMERATE)
	};
	($getter:expr, $setter:expr) => {
		property_spec_getter_setter!($getter, $setter, ::std::stringify!($getter))
	};
}
