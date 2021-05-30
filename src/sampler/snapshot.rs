use crate::core::initialize::initialize;
use crate::core::types::Pid;
use anyhow::{Error, Result};

pub fn snapshot(pid: Pid, lock_process: bool) -> Result<(), Error> {
    let mut getter = initialize(pid, lock_process)?;
    let trace = getter.get_trace()?;
    for x in trace.iter().rev() {
        println!("{}", x);
    }
    Ok(())
}
