/// Core types used throughout rbspy: StackFrame and StackTrace
use std::cmp::Ordering;
use std::fmt;
use std::time::SystemTime;
use std::{self, convert::From};

use anyhow::{Error, Result};
use thiserror::Error;

pub use remoteprocess::{Pid, Process, ProcessMemory};

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
    #[error("Permission error")]
    PermissionError,
    #[error("Couldn't lock the process")]
    ProcessNotLocked,
    #[error("Copy error: {}", _0)]
    Message(String),
    #[error("Too much memory requested when copying: {}", _0)]
    RequestTooLarge(usize),
    #[error("Tried to read invalid string")]
    InvalidStringError(std::string::FromUtf8Error),
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
    pub fn iter(&self) -> std::slice::Iter<StackFrame> {
        self.trace.iter()
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
            /* On Mac, 60 seems to correspond to the process ended */
            /* On Windows, 299 happens when the process ended */
            Some(3) | Some(60) | Some(299) => MemoryCopyError::ProcessEnded,
            // On *nix EFAULT means that the address was invalid
            Some(14) => MemoryCopyError::InvalidAddressError(addr),
            _ => MemoryCopyError::Io(addr, error),
        }
    }
}
pub trait ProcessRetry {
    fn new_with_retry(pid: Pid) -> Result<Process>;
}

impl ProcessRetry for remoteprocess::Process {
    // It can take a moment for the ruby process to spin up, so new_with_retry automatically
    // retries for a few seconds. This delay mostly seems to affect macOS and Windows and is
    // especially common in CI environments.
    fn new_with_retry(pid: Pid) -> Result<Process> {
        let retry_interval = std::time::Duration::from_millis(10);
        let mut retries = 500;
        loop {
            match Process::new(pid) {
                Ok(p) => return Ok(p),
                Err(e) => {
                    if retries == 0 {
                        return Err(e)?;
                    }
                    std::thread::sleep(retry_interval);
                    retries -= 1;
                }
            }
        }
    }
}
