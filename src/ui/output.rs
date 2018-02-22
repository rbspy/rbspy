#[cfg(test)]
extern crate tempdir;

use failure::Error;
use std::fs::File;

use ui::callgrind;
use ui::summary;
use ui::flamegraph;
use core::types::StackTrace;

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
        self.0.add(&stack.trace);
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
        self.0.add_function_name(&stack.trace);
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
        self.0.add_lineno(&stack.trace);
        Ok(())
    }

    fn complete(&mut self, mut file: File) -> Result<(), Error> {
        self.0.write(&mut file)?;
        Ok(())
    }
}
