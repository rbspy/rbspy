#[cfg(test)]
extern crate tempdir;

use failure::Error;
use std::fs::File;

use crate::ui::{callgrind, flamegraph, speedscope, summary};
use crate::core::types::{StackTrace, StackFrame};

pub trait Outputter {
    fn record(&mut self, stack: &StackTrace) -> Result<(), Error>;
    fn complete(&mut self, file: File) -> Result<(), Error>;
}

// Uses Inferno to visualize stack traces
pub struct Flamegraph(pub flamegraph::Stats);

impl Outputter for Flamegraph {
    fn record(&mut self, stack: &StackTrace) -> Result<(), Error> {
        self.0.record(&stack.trace)?;
        Ok(())
    }

    fn complete(&mut self, file: File) -> Result<(), Error> {
        self.0.write(file)?;
        Ok(())
    }
}

pub struct Callgrind(pub callgrind::Stats);

impl Outputter for Callgrind {
    fn record(&mut self, stack: &StackTrace) -> Result<(), Error> {
        self.0.add(&filter_unknown(&stack).trace);
        Ok(())
    }

    fn complete(&mut self, mut file: File) -> Result<(), Error> {
        self.0.finish();
        self.0.write(&mut file)?;
        Ok(())
    }
}

pub struct Summary(pub summary::Stats);

impl Outputter for Summary {
    fn record(&mut self, stack: &StackTrace) -> Result<(), Error> {
        self.0.add_function_name(&filter_unknown(&stack));
        Ok(())
    }

    fn complete(&mut self, mut file: File) -> Result<(), Error> {
        self.0.write(&mut file)?;
        Ok(())
    }
}

pub struct SummaryLine(pub summary::Stats);

impl Outputter for SummaryLine {
    fn record(&mut self, stack: &StackTrace) -> Result<(), Error> {
        self.0.add_lineno(&filter_unknown(&stack));
        Ok(())
    }

    fn complete(&mut self, mut file: File) -> Result<(), Error> {
        self.0.write(&mut file)?;
        Ok(())
    }
}

pub struct Speedscope(pub speedscope::Stats);

impl Outputter for Speedscope {
    fn record(&mut self, stack: &StackTrace) -> Result<(), Error> {
        self.0.record(&stack)?;
        Ok(())
    }

    fn complete(&mut self, file: File) -> Result<(), Error> {
        self.0.write(file)?;
        Ok(())
    }
}

/// Filter out unknown functions from stack trace before reporting.
/// Most of the time it isn't useful to include the "unknown C function" stacks.
fn filter_unknown(stack: &StackTrace) -> StackTrace {
    let unknown = StackFrame::unknown_c_function();
    let vec: Vec<StackFrame> = stack.trace.iter().filter(|&x| x != &unknown).map(|x| x.clone()).collect();
    let filtered = if vec.is_empty() {
        vec!(unknown)
    } else {
        vec
    };
    StackTrace {
        trace: filtered,
        pid: stack.pid,
        thread_id: stack.thread_id,
        on_cpu: stack.on_cpu,
    }
}


