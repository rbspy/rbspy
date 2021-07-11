extern crate anyhow;
#[cfg(test)]
extern crate byteorder;
extern crate chrono;
#[macro_use]
extern crate clap;
extern crate ctrlc;
extern crate elf;
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
extern crate tempdir;
extern crate term_size;
#[cfg(windows)]
extern crate winapi;

use anyhow::{Error, Result};

use std::fs::File;
use std::path::PathBuf;

mod core;
pub mod recorder;
mod storage;
mod ui;

pub use crate::core::types::OutputFormat;
pub use crate::core::types::Pid;

pub fn report(format: OutputFormat, input: PathBuf, output: PathBuf) -> Result<(), Error> {
    let input_file = File::open(input)?;
    let stuff = storage::from_reader(input_file)?.traces;
    let mut outputter = format.outputter(0.1);
    for trace in stuff {
        outputter.record(&trace)?;
    }
    if output.display().to_string() == "-" {
        outputter.complete(&mut std::io::stdout())?;
    } else {
        outputter.complete(&mut File::create(output)?)?;
    }
    Ok(())
}
