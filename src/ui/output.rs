#[cfg(test)]
extern crate tempdir;

use std::fs::File;

use crate::core::types::{StackFrame, StackTrace};
use crate::ui::{callgrind, flamegraph, speedscope, summary};

use anyhow::Result;

pub trait Outputter {
    fn record(&mut self, stack: &StackTrace) -> Result<()>;
    fn complete(&mut self, file: File) -> Result<()>;
}

// Uses Inferno to visualize stack traces
pub struct Flamegraph(pub flamegraph::Stats);

impl Outputter for Flamegraph {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.record(&stack.trace)?;
        Ok(())
    }

    fn complete(&mut self, file: File) -> Result<()> {
        self.0.write(file)?;
        Ok(())
    }
}

pub struct Callgrind(pub callgrind::Stats);

impl Outputter for Callgrind {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.add(&filter_unknown(&stack.trace));
        Ok(())
    }

    fn complete(&mut self, mut file: File) -> Result<()> {
        self.0.finish();
        self.0.write(&mut file)?;
        Ok(())
    }
}

pub struct Summary(pub summary::Stats);

impl Outputter for Summary {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.add_function_name(&filter_unknown(&stack.trace));
        Ok(())
    }

    fn complete(&mut self, mut file: File) -> Result<()> {
        self.0.write(&mut file)?;
        Ok(())
    }
}

pub struct SummaryLine(pub summary::Stats);

impl Outputter for SummaryLine {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.add_lineno(&filter_unknown(&stack.trace));
        Ok(())
    }

    fn complete(&mut self, mut file: File) -> Result<()> {
        self.0.write(&mut file)?;
        Ok(())
    }
}

pub struct Speedscope(pub speedscope::Stats);

impl Outputter for Speedscope {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.record(&stack)?;
        Ok(())
    }

    fn complete(&mut self, file: File) -> Result<()> {
        self.0.write(file)?;
        Ok(())
    }
}

/// Filter out unknown functions from stack trace before reporting.
/// Most of the time it isn't useful to include the "unknown C function" stacks.
fn filter_unknown(trace: &[StackFrame]) -> Vec<StackFrame> {
    let unknown = StackFrame::unknown_c_function();
    let vec: Vec<StackFrame> = trace.iter().filter(|&x| x != &unknown).cloned().collect();
    if vec.is_empty() {
        vec![unknown]
    } else {
        vec
    }
}
