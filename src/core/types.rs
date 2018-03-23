/// Core types used throughout rbspy: StackFrame and StackTrace

use std::cmp::Ordering;
use std::fmt;
use std;

#[cfg(unix)]
pub use libc::pid_t;

#[cfg(windows)]
pub type pid_t = u32;

use read_process_memory::*;

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
    // these are both options for backwards compatibility with older rbspy saved data that didn't
    // have PID / cpu data
    pub pid: Option<pid_t>,
    pub thread_id: Option<usize>,
    pub on_cpu: Option<bool>,
}

pub struct Process<T> where T: CopyAddress {
    pub pid: Option<pid_t>,
    pub source: T,
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
