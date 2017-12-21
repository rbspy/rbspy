#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod ruby_bindings {

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

}
