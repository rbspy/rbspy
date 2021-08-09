#[cfg(test)]
extern crate tempdir;

use std::io::Write;

use crate::core::types::{StackFrame, StackTrace};
use crate::ui::{callgrind, flamegraph, speedscope, summary};

use anyhow::Result;

pub trait Outputter {
    fn record(&mut self, stack: &StackTrace) -> Result<()>;
    fn complete(&mut self, write: &mut dyn Write) -> Result<()>;
}

// Uses Inferno to visualize stack traces
pub struct Flamegraph {
    stats: flamegraph::Stats,
    min_width: f64,
}

impl Outputter for Flamegraph {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.stats.record(&stack.trace)?;
        Ok(())
    }

    fn complete(&mut self, write: &mut dyn Write) -> Result<()> {
        self.stats.write_flamegraph(write, self.min_width)?;
        Ok(())
    }
}

impl Flamegraph {
    pub fn new(min_width: f64) -> Flamegraph {
        Flamegraph {
            min_width: min_width,
            stats: Default::default(),
        }
    }
}

// Collapsed stacks are the intermediate flamegraph format,
// useful for making additional processing or using other flamegraph generators.
#[derive(Default)]
pub struct Collapsed(pub flamegraph::Stats);

impl Outputter for Collapsed {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.record(&stack.trace)?;
        Ok(())
    }

    fn complete(&mut self, mut write: &mut dyn Write) -> Result<()> {
        self.0.write_collapsed(&mut write)?;
        Ok(())
    }
}

pub struct Callgrind(pub callgrind::Stats);

impl Outputter for Callgrind {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.add(&filter_unknown(&stack.trace));
        Ok(())
    }

    fn complete(&mut self, mut write: &mut dyn Write) -> Result<()> {
        self.0.finish();
        self.0.write(&mut write)?;
        Ok(())
    }
}

pub struct Summary(pub summary::Stats);

impl Outputter for Summary {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.add_function_name(&filter_unknown(&stack.trace));
        Ok(())
    }

    fn complete(&mut self, mut write: &mut dyn Write) -> Result<()> {
        self.0.write(&mut write)?;
        Ok(())
    }
}

pub struct SummaryLine(pub summary::Stats);

impl Outputter for SummaryLine {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.add_lineno(&filter_unknown(&stack.trace));
        Ok(())
    }

    fn complete(&mut self, mut write: &mut dyn Write) -> Result<()> {
        self.0.write(&mut write)?;
        Ok(())
    }
}

pub struct Speedscope(pub speedscope::Stats);

impl Outputter for Speedscope {
    fn record(&mut self, stack: &StackTrace) -> Result<()> {
        self.0.record(&stack)?;
        Ok(())
    }

    fn complete(&mut self, write: &mut dyn Write) -> Result<()> {
        self.0.write(write)?;
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
