#![allow(non_camel_case_types)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use super::ShadowType;
use std::ffi::{CString, NulError};
use std::ptr;

pub fn create_label(input_label_name: &str) -> Result<ShadowType, NulError> {
    let input_label_name = CString::new(input_label_name)?;

    unsafe {
        Ok(dfsan_create_label(
            input_label_name.as_ptr(),
            ptr::null_mut(),
        ))
    }
}

pub fn has_label(value_label: ShadowType, query_label: ShadowType) -> bool {
    unsafe { dfsan_has_label(value_label, query_label) != 0 }
}
