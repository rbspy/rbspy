use crate::core::process::Pid;
use crate::core::ruby_spy::RubySpy;
use crate::core::types::StackTrace;
use anyhow::{Error, Result};

/// Captures a single trace from the process belonging to `pid`
pub fn snapshot(
    pid: Pid,
    lock_process: bool,
    force_version: Option<String>,
) -> Result<StackTrace, Error> {
    RubySpy::retry_new(pid, 10, force_version)?.get_stack_trace(lock_process)
}
