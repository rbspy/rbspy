use crate::core::initialize::initialize;
use crate::core::types::{Pid, StackTrace};
use anyhow::{Error, Result};

/// Captures a single trace from the process belonging to `pid`
pub fn snapshot(pid: Pid, lock_process: bool) -> Result<StackTrace, Error> {
    let mut getter = initialize(pid, lock_process)?;
    getter.get_trace()
}
