use crate::core::initialize::initialize;
use crate::core::types::Pid;
use anyhow::{Error, Result};

/// Captures a single trace from the process belonging to `pid`
pub fn snapshot(pid: Pid, lock_process: bool) -> Result<String, Error> {
    let mut getter = initialize(pid, lock_process)?;
    let trace = getter.get_trace()?;
    let frames: Vec<String> = trace.iter().rev().map(|s| s.to_string()).collect();
    Ok(frames.join("\n"))
}
