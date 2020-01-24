/// Core types used throughout rbspy: StackFrame and StackTrace

use std::cmp::Ordering;
use std::fmt;
use std::{self, convert::From};
use std::time::SystemTime;

use failure::Context;

pub use remoteprocess::{Error, Process, Pid, ProcessMemory};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Header {
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

#[derive(Fail, Debug)]
pub enum MemoryCopyError {
    #[fail(display = "Permission denied when reading from process. If you're not running as root, try again with sudo. If you're using Docker, try passing `--cap-add=SYS_PTRACE` to `docker run`")]

    PermissionDenied,
    #[fail(display = "Failed to copy memory address {:x}", _0)] Io(usize, #[cause] std::io::Error),
    #[fail(display = "Process isn't running")] ProcessEnded,
    #[fail(display = "Copy error: {}", _0)] Message(String),
    #[fail(display = "Too much memory requested when copying: {}", _0)] RequestTooLarge(usize),
    #[fail(display = "Tried to read invalid string")]
    InvalidStringError(#[cause] std::string::FromUtf8Error),
    #[fail(display = "Tried to read invalid memory address {:x}", _0)]
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
            name: "<c function>".to_string(),
            relative_path: "unknown".to_string(),
            absolute_path: None,
            lineno: 0
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
    pub fn iter(&self) ->  std::slice::Iter<StackFrame> {
        self.trace.iter()
    }
}

impl From<Context<usize>> for MemoryCopyError {
    fn from(context: Context<usize>) -> Self {
        let addr = *context.get_context();
        let error = std::io::Error::last_os_error();

        if error.kind() == std::io::ErrorKind::PermissionDenied {
            return MemoryCopyError::PermissionDenied;
        }

        return match error.raw_os_error() {
            /* On Mac, 60 seems to correspond to the process ended */
            /* On Windows, 299 happens when the process ended */
            Some(3) | Some(60) | Some(299) => {
                MemoryCopyError::ProcessEnded
            },
            // On *nix EFAULT means that the address was invalid
            Some(14) => MemoryCopyError::InvalidAddressError(addr),
            _ => MemoryCopyError::Io(addr, error)
        }
    }
}
