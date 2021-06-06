/// Core types used throughout rbspy: StackFrame and StackTrace
use std::cmp::Ordering;
use std::fmt;
use std::time::SystemTime;
use std::{self, convert::From};

use anyhow::Error;
use clap::ArgEnum;
use remoteprocess::Pid;
use thiserror::Error;

use crate::ui::*;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub(crate) struct Header {
    pub sample_rate: Option<u32>,
    pub rbspy_version: Option<String>,
    pub start_time: Option<SystemTime>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub name: String,
    pub relative_path: String,
    pub absolute_path: Option<String>,
    pub lineno: u32,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct StackTrace {
    pub trace: Vec<StackFrame>,
    pub pid: Option<Pid>,
    pub thread_id: Option<usize>,
    pub time: Option<SystemTime>,
}

#[derive(Error, Debug)]
pub enum MemoryCopyError {
    #[error("The operation completed successfully")]
    OperationSucceeded,
    #[error("Permission denied when reading from process. If you're not running as root, try again with sudo. If you're using Docker, try passing `--cap-add=SYS_PTRACE` to `docker run`")]
    PermissionDenied,
    #[error("Failed to copy memory address {:x}", _0)]
    Io(usize, std::io::Error),
    #[error("Process isn't running")]
    ProcessEnded,
    #[error("Copy error: {}", _0)]
    Message(String),
    #[error("Tried to read invalid memory address {:x}", _0)]
    InvalidAddressError(usize),
}

impl StackFrame {
    pub fn path(&self) -> &str {
        match self.absolute_path {
            Some(ref p) => p.as_ref(),
            None => self.relative_path.as_ref(),
        }
    }

    // we use this stack frame when there's a C function that we don't recognize in the stack. This
    // would be a constant but it has strings in it so it can't be.
    pub fn unknown_c_function() -> StackFrame {
        StackFrame {
            name: "(unknown) [c function]".to_string(),
            relative_path: "(unknown)".to_string(),
            absolute_path: None,
            lineno: 0,
        }
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - {}:{}", self.name, self.path(), self.lineno)
    }
}

impl Ord for StackFrame {
    fn cmp(&self, other: &StackFrame) -> Ordering {
        self.path()
            .cmp(other.path())
            .then(self.name.cmp(&other.name))
            .then(self.lineno.cmp(&other.lineno))
    }
}

impl PartialOrd for StackFrame {
    fn partial_cmp(&self, other: &StackFrame) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl StackTrace {
    pub fn new_empty() -> StackTrace {
        StackTrace {
            pid: None,
            trace: Vec::new(),
            thread_id: None,
            time: None,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<StackFrame> {
        self.trace.iter()
    }
}

impl fmt::Display for StackTrace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let frames: Vec<String> = self.iter().rev().map(|s| s.to_string()).collect();
        write!(f, "{}", frames.join("\n"))
    }
}

impl From<Error> for MemoryCopyError {
    fn from(error: Error) -> Self {
        let addr = *error.downcast_ref::<usize>().unwrap_or(&0);
        let error = std::io::Error::last_os_error();

        if error.kind() == std::io::ErrorKind::PermissionDenied {
            return MemoryCopyError::PermissionDenied;
        }

        match error.raw_os_error() {
            // Sometimes Windows returns this error code
            Some(0) => MemoryCopyError::OperationSucceeded,
            // On *nix EFAULT means that the address was invalid
            Some(14) => MemoryCopyError::InvalidAddressError(addr),
            _ => MemoryCopyError::Io(addr, error),
        }
    }
}

/// File formats into which rbspy can convert its recorded traces

// The values of this enum get translated directly to command line arguments. Make them
// lowercase so that we don't have camelcase command line arguments
#[derive(ArgEnum, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[allow(non_camel_case_types)]
pub enum OutputFormat {
    flamegraph,
    collapsed,
    callgrind,
    speedscope,
    pprof,
    summary,
    summary_by_line,
}

impl OutputFormat {
    pub fn outputter(self, flame_min_width: f64) -> Box<dyn output::Outputter> {
        match self {
            OutputFormat::flamegraph => Box::new(output::Flamegraph::new(flame_min_width)),
            OutputFormat::collapsed => Box::new(output::Collapsed::default()),
            OutputFormat::callgrind => Box::new(output::Callgrind(callgrind::Stats::new())),
            OutputFormat::speedscope => Box::new(output::Speedscope(speedscope::Stats::new())),
            OutputFormat::pprof => Box::new(output::Pprof(pprof::Stats::new())),
            OutputFormat::summary => Box::new(output::Summary(summary::Stats::new())),
            OutputFormat::summary_by_line => Box::new(output::SummaryLine(summary::Stats::new())),
        }
    }

    pub fn extension(&self) -> String {
        match *self {
            OutputFormat::flamegraph => "flamegraph.svg",
            OutputFormat::collapsed => "collapsed.txt",
            OutputFormat::callgrind => "callgrind.txt",
            OutputFormat::speedscope => "speedscope.json",
            OutputFormat::pprof => "profile.pb.gz",
            OutputFormat::summary => "summary.txt",
            OutputFormat::summary_by_line => "summary_by_line.txt",
        }
        .to_string()
    }

    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        Self::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "flamegraph" => Ok(OutputFormat::flamegraph),
            "collapsed" => Ok(OutputFormat::collapsed),
            "callgrind" => Ok(OutputFormat::callgrind),
            "speedscope" => Ok(OutputFormat::speedscope),
            "pprof" => Ok(OutputFormat::pprof),
            "summary" => Ok(OutputFormat::summary),
            "summary-by-line" => Ok(OutputFormat::summary_by_line),
            _ => Err(anyhow::format_err!("Unknown output format: {}", s)),
        }
    }
}
