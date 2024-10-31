extern crate anyhow;
#[cfg(test)]
extern crate byteorder;
extern crate chrono;
extern crate clap;
extern crate ctrlc;
extern crate env_logger;
extern crate inferno;
extern crate libc;
#[cfg(target_os = "macos")]
extern crate libproc;
#[cfg(unix)]
extern crate nix;
extern crate proc_maps;
#[macro_use]
extern crate log;
extern crate rand;
#[cfg(test)]
extern crate rbspy_testdata;
extern crate remoteprocess;

extern crate rbspy_ruby_structs as bindings;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate terminal_size;
#[cfg(windows)]
extern crate winapi;

use core::ruby_spy::RubySpy;

use anyhow::Result;

mod core;
pub mod recorder;
pub mod sampler;
mod storage;
pub mod ui;

pub use crate::core::process::Pid;
pub use crate::core::types::OutputFormat;
pub use crate::core::types::StackFrame;
pub use crate::core::types::StackTrace;

/// Generate visualization (e.g. a flamegraph) from raw data that was previously recorded by rbspy
pub fn report(
    format: OutputFormat,
    input: &mut dyn std::io::Read,
    output: &mut dyn std::io::Write,
) -> Result<()> {
    let traces = storage::from_reader(input)?.traces;
    let mut outputter = format.outputter(0.1);
    for trace in traces {
        outputter.record(&trace)?;
    }
    outputter.complete(output)?;
    Ok(())
}

pub fn inspect(pid: Pid, force_version: Option<String>) -> Result<()> {
    let ruby_spy = RubySpy::new(pid, force_version)?;
    let vm = ruby_spy.inspect();
    println!("Ruby version: {}", vm.ruby_version.semver_version);
    println!("Ruby VM address: {:#x}", vm.ruby_vm_addr_location);
    println!(
        "Current thread address: {:#x}",
        vm.current_thread_addr_location
    );
    println!(
        "Global symbols address: {:#x}",
        vm.global_symbols_addr_location.unwrap_or(0)
    );

    Ok(())
}
