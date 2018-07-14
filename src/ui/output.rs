#[cfg(test)]
extern crate tempdir;

use failure::Error;
use std::fs::File;

use ui::callgrind;
use ui::summary;
use ui::flamegraph;
use ui::speedscope;
use core::types::{StackTrace, StackFrame};

pub trait Outputter {
    fn record(&mut self, stack: &StackTrace) -> Result<(), Error>;
    fn complete(&mut self, file: File) -> Result<(), Error>;
}

// Uses Brendan Gregg's flamegraph.pl script (which we vendor) to visualize stack traces
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
        self.0.add(&filter_unknown(&stack.trace));
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
        self.0.add_function_name(&filter_unknown(&stack.trace));
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
        self.0.add_lineno(&filter_unknown(&stack.trace));
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
        self.0.record(&stack.trace)?;
        Ok(())
    }

    fn complete(&mut self, file: File) -> Result<(), Error> {
        self.0.write(file)?;
        Ok(())
    }
}

/// Filter out unknown functions from stack trace before reporting.
/// Most of the time it isn't useful to include the "unknown C function" stacks.
fn filter_unknown(trace: &Vec<StackFrame>) -> Vec<StackFrame> {
    let unknown = StackFrame::unknown_c_function();
    let vec: Vec<StackFrame> = trace.iter().filter(|&x| x != &unknown).map(|x| x.clone()).collect();
    if vec.len() == 0 {
        vec!(unknown)
    } else {
        vec
    }
}


