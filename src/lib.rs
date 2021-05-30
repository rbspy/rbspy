#![cfg_attr(rustc_nightly, feature(test))]

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
#[cfg(windows)]
use winapi::um::timeapi;

mod core;
pub mod sampler;
mod storage;
mod ui;

pub use crate::core::types::OutputFormat;
pub use crate::core::types::Pid;

#[cfg(target_os = "macos")]
pub fn check_root_user() -> bool {
    nix::unistd::Uid::effective().is_root()
}

#[cfg(all(windows, target_arch = "x86_64"))]
fn check_wow64_process(pid: Pid) {
    if is_wow64_process(pid).unwrap() {
        eprintln!("Unable to profile 32-bit Ruby with 64-bit rbspy.");
        std::process::exit(1);
    }
}

#[cfg(all(windows, target_arch = "x86_64"))]
fn is_wow64_process(pid: Pid) -> Result<bool, Error> {
    use std::os::windows::io::RawHandle;
    use winapi::shared::minwindef::{BOOL, FALSE, PBOOL};
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winnt::PROCESS_QUERY_INFORMATION;
    use winapi::um::wow64apiset::IsWow64Process;

    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid) };

    if handle == (0 as RawHandle) {
        return Err(format_err!(
            "Unable to fetch process handle for process {}",
            pid
        ));
    }

    let mut is_wow64: BOOL = 0;

    if unsafe { IsWow64Process(handle, &mut is_wow64 as PBOOL) } == FALSE {
        return Err(format_err!("Could not determine process bitness! {}", pid));
    }

    Ok(is_wow64 != 0)
}

#[test]
#[cfg(all(windows, target_arch = "x86_64"))]
fn test_is_wow64_process() {
    let programs = vec![
        "C:\\Program Files (x86)\\Internet Explorer\\iexplore.exe",
        "C:\\Program Files\\Internet Explorer\\iexplore.exe",
    ];

    let results: Vec<bool> = programs
        .iter()
        .map(|path| {
            let mut cmd = Command::new(path)
                .spawn()
                .expect("ls command failed to start");

            let result = is_wow64_process(cmd.id());

            cmd.kill()
                .expect("command wasn't running or couldn't be killed");

            result.unwrap()
        })
        .collect();

    assert_eq!(results, vec![true, false]);
}

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
