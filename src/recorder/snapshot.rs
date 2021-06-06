use crate::core::initialize::initialize;
use crate::core::process::Pid;
use crate::core::types::StackTrace;
use anyhow::{Error, Result};

/// Captures a single trace from the process belonging to `pid`
pub fn snapshot(
    pid: Pid,
    lock_process: bool,
    force_version: Option<String>,
    on_cpu: bool,
) -> Result<StackTrace, Error> {
    let mut getter = initialize(pid, lock_process, force_version, on_cpu)?;
    Ok(getter.get_trace()?.unwrap_or(StackTrace::new_empty()))
}
