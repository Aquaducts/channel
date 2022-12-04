#![allow(improper_ctypes)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::unreadable_literal)]

pub mod command;
pub mod config;
pub mod container;
pub mod io;
pub mod lxc {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[macro_export]
macro_rules! make_str {
    ($s:expr) => {
        std::ffi::CString::new($s).unwrap().as_ptr()
    };
}
