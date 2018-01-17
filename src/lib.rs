#![cfg_attr(rustc_nightly, feature(test))]

#[macro_use]
extern crate log;

#[cfg(test)]
extern crate byteorder;
extern crate elf;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate libc;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;
extern crate read_process_memory;
#[cfg(target_os = "macos")]
extern crate regex;
extern crate ruby_bindings as bindings;

pub mod proc_maps;
pub mod address_finder;
pub mod copy;
pub mod stack_trace;
pub mod test_utils;
