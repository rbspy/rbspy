use anyhow::Result;
use inferno::flamegraph::{Direction, Options};
use std::collections::HashMap;
use std::io::Write;

use crate::core::types::StackFrame;

// Simple counter that maps stacks to flamegraph collapsed format
#[derive(Default)]
pub struct Stats {
    pub counts: HashMap<String, usize>,
}

impl Stats {
    pub fn record(&mut self, stack: &[StackFrame]) -> Result<()> {
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
            opts.hash = true;
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

    // Build test stats
    fn build_stats() -> Result<Stats> {
        let mut stats = Stats::default();
        stats.record(&vec![f(1)])?;
        stats.record(&vec![f(2), f(1)])?;
        stats.record(&vec![f(2), f(1)])?;
        stats.record(&vec![f(2), f(3), f(1)])?;
        stats.record(&vec![f(2), f(3), f(1)])?;
        stats.record(&vec![f(2), f(3), f(1)])?;
        Ok(stats)
    }

    fn assert_contains(counts: &HashMap<String, usize>, s: &str, val: usize) {
        assert_eq!(counts.get(&s.to_string()), Some(&val));
    }

    #[test]
    fn test_stats() -> Result<()> {
        let stats = build_stats()?;
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

    #[test]
    fn test_collapsed() -> Result<()> {
        let stats = build_stats()?;
        let mut writer = Cursor::new(Vec::<u8>::new());
        stats.write_collapsed(&mut writer)?;
        let collapsed_text = std::str::from_utf8(writer.get_ref())?;
        assert!(collapsed_text.contains("func1 - file1.rb:1 1"));
        assert!(
            collapsed_text.contains("func1 - file1.rb:1;func3 - file3.rb:3;func2 - file2.rb:2 3")
        );
        assert!(collapsed_text.contains("func1 - file1.rb:1;func2 - file2.rb:2 2"));

        Ok(())
    }

    #[test]
    fn test_flamegraph_from_collapsed() -> Result<()> {
        let stats = build_stats()?;

        let mut writer = Cursor::new(Vec::<u8>::new());
        stats.write_collapsed(&mut writer)?;

        let collapsed_reader = Cursor::new(writer.into_inner());
        let svg_writer = Cursor::new(Vec::new());

        let mut opts = Options::default();
        opts.direction = Direction::Inverted;
        opts.min_width = 0.1;
        inferno::flamegraph::from_reader(&mut opts, collapsed_reader, svg_writer)?;

        Ok(())
    }
}
