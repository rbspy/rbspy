use std::collections::HashMap;
use std::io;
use std::fs::File;

use crate::core::types::StackFrame;

use failure::Error;
use inferno::flamegraph::{Direction, Options};

pub struct Stats {
    pub counts: HashMap<String, usize>,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            counts: HashMap::new(),
        }
    }

    pub fn record(&mut self, stack: &[StackFrame]) -> Result<(), io::Error> {
        let frame = stack.iter().rev().map(|frame| {
            format!("{}", frame)
        }).collect::<Vec<String>>().join(";");

        *self.counts.entry(frame).or_insert(0) += 1;
        Ok(())
    }

    pub fn write(&self, w: File) -> Result<(), Error> {
        let lines: Vec<String> = self.counts.iter().map(|(k, v)| format!("{} {}", k, v)).collect();

        let mut opts =  Options {
            direction: Direction::Inverted,
            min_width: 2_f64,
            ..Default::default()
        };

        inferno::flamegraph::from_lines(&mut opts, lines.iter().map(|x| x.as_str()), w).unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::flamegraph::*;

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
        let mut stats = Stats::new();

        stats.record(&vec![f(1)])?;
        stats.record(&vec![f(2), f(1)])?;
        stats.record(&vec![f(2), f(1)])?;
        stats.record(&vec![f(2), f(3), f(1)])?;
        stats.record(&vec![f(2), f(3), f(1)])?;
        stats.record(&vec![f(2), f(3), f(1)])?;

        let counts = &stats.counts;
        assert_contains(counts, "func1 - file1.rb:1", 1);
        assert_contains(counts, "func1 - file1.rb:1;func3 - file3.rb:3;func2 - file2.rb:2", 3);
        assert_contains(counts, "func1 - file1.rb:1;func2 - file2.rb:2", 2);

        Ok(())
    }
}
