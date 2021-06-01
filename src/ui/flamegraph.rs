use std::collections::HashMap;
use std::io::{self, Write};

use crate::core::types::StackFrame;

use anyhow::Result;
use inferno::flamegraph::{Direction, Options};

// Simple counter that maps stacks to flamegraph collapsed format
#[derive(Default)]
pub struct Stats {
    pub counts: HashMap<String, usize>,
}

impl Stats {
    pub fn new() -> Stats {
        Default::default()
    }

    pub fn record(&mut self, stack: &[StackFrame]) -> Result<(), io::Error> {
        let frame = stack
            .iter()
            .rev()
            .map(|frame| format!("{}", frame))
            .collect::<Vec<String>>()
            .join(";");

        *self.counts.entry(frame).or_insert(0) += 1;
        Ok(())
    }

    pub fn write_flamegraph<W: Write>(&self, w: W, min_width: f64) -> Result<()> {
        if self.is_empty() {
            eprintln!("Warning: no profile samples were collected");
        } else {
            let mut opts = Options::default();
            opts.direction = Direction::Inverted;
            opts.min_width = min_width;
            inferno::flamegraph::from_lines(
                &mut opts,
                self.get_lines().iter().map(|x| x.as_str()),
                w,
            )?;
        }

        Ok(())
    }

    pub fn write_collapsed<W: Write>(&self, w: &mut W) -> Result<()> {
        if self.is_empty() {
            eprintln!("Warning: no profile samples were collected");
        } else {
            self.get_lines()
                .iter()
                .try_for_each(|line| write!(w, "{}\n", line))?;
        }
        Ok(())
    }

    fn get_lines(&self) -> Vec<String> {
        self.counts
            .iter()
            .map(|(frame, count)| format!("{} {}", frame, count))
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::flamegraph::*;
    use std::io::Cursor;

    // Build a test stackframe
    fn f(i: u32) -> StackFrame {
        StackFrame {
            name: format!("func{}", i),
            relative_path: format!("file{}.rb", i),
            absolute_path: None,
            lineno: i,
        }
    }

    fn assert_contains(counts: &HashMap<String, usize>, s: &str, val: usize) {
        assert_eq!(counts.get(&s.to_string()), Some(&val));
    }

    #[test]
    fn test_stats() -> Result<(), io::Error> {
        let mut stats = Stats::new(0.1);

        stats.record(&vec![f(1)])?;
        stats.record(&vec![f(2), f(1)])?;
        stats.record(&vec![f(2), f(1)])?;
        stats.record(&vec![f(2), f(3), f(1)])?;
        stats.record(&vec![f(2), f(3), f(1)])?;
        stats.record(&vec![f(2), f(3), f(1)])?;

        let counts = &stats.counts;
        assert_contains(counts, "func1 - file1.rb:1", 1);
        assert_contains(
            counts,
            "func1 - file1.rb:1;func3 - file3.rb:3;func2 - file2.rb:2",
            3,
        );
        assert_contains(counts, "func1 - file1.rb:1;func2 - file2.rb:2", 2);

        Ok(())
    }
}
