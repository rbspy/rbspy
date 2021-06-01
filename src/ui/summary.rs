use std::collections::{HashMap, HashSet};
use std::io;

use crate::core::types::StackFrame;

struct Counts {
    self_: u64,
    total: u64,
}

#[derive(Default)]
pub struct Stats {
    counts: HashMap<String, Counts>,
    total_traces: u32,
}

impl Stats {
    const HEADER: &'static str = "% self  % total  name";

    pub fn new() -> Stats {
        Default::default()
    }

    fn inc_self(&mut self, name: String) {
        let entry = self
            .counts
            .entry(name)
            .or_insert(Counts { self_: 0, total: 0 });
        entry.self_ += 1;
    }

    fn inc_tot(&mut self, name: String) {
        let entry = self
            .counts
            .entry(name)
            .or_insert(Counts { self_: 0, total: 0 });
        entry.total += 1;
    }

    fn name_function(frame: &StackFrame) -> String {
        format!("{} - {}", frame.name, frame.relative_path)
    }

    fn name_lineno(frame: &StackFrame) -> String {
        format!("{}", frame)
    }

    // Aggregate by function name
    pub fn add_function_name(&mut self, stack: &[StackFrame]) {
        if stack.is_empty() {
            return;
        }
        self.total_traces += 1;
        self.inc_self(Stats::name_function(&stack[0]));
        let mut set: HashSet<String> = HashSet::new();
        for frame in stack {
            set.insert(Stats::name_function(frame));
        }
        for name in set.into_iter() {
            self.inc_tot(name);
        }
    }

    // Aggregate by function name + line number
    pub fn add_lineno(&mut self, stack: &[StackFrame]) {
        if stack.is_empty() {
            return;
        }
        self.total_traces += 1;
        self.inc_self(Stats::name_lineno(&stack[0]));
        let mut set: HashSet<&StackFrame> = HashSet::new();
        for frame in stack {
            set.insert(&frame);
        }
        for frame in set {
            self.inc_tot(Stats::name_lineno(frame));
        }
    }

    pub fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        self.write_counts(w, None, None)
    }

    pub fn print_top_n(&self, n: usize, truncate: Option<usize>) -> io::Result<()> {
        self.write_counts(&mut ::std::io::stdout(), Some(n), truncate)
    }

    fn write_counts(
        &self,
        w: &mut dyn io::Write,
        top: Option<usize>,
        truncate: Option<usize>,
    ) -> io::Result<()> {
        let top = top.unwrap_or(::std::usize::MAX);
        let truncate = truncate.unwrap_or(::std::usize::MAX);
        let mut sorted: Vec<(u64, u64, &str)> = self
            .counts
            .iter()
            .map(|(x, y)| (y.self_, y.total, x.as_ref()))
            .collect();
        sorted.sort_unstable();
        let counts = sorted.iter().rev().take(top);
        writeln!(w, "{}", Stats::HEADER)?;
        for &(self_, total, name) in counts {
            writeln!(
                w,
                "{:>6.2} {:>8.2}  {:.*}",
                100.0 * (self_ as f64) / f64::from(self.total_traces),
                100.0 * (total as f64) / f64::from(self.total_traces),
                truncate - 14 - 3,
                name
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::summary::*;

    // Build a test stackframe
    fn f(i: u32) -> StackFrame {
        StackFrame {
            name: format!("func{}", i),
            relative_path: format!("file{}.rb", i),
            absolute_path: None,
            lineno: i,
        }
    }

    #[test]
    fn stats_by_function() {
        let mut stats = Stats::new();

        stats.add_function_name(&vec![f(1)]);
        stats.add_function_name(&vec![f(3), f(2), f(1)]);
        stats.add_function_name(&vec![f(2), f(1)]);
        stats.add_function_name(&vec![f(3), f(1)]);
        stats.add_function_name(&vec![f(2), f(3), f(1)]);

        let expected = "% self  % total  name
 40.00    60.00  func3 - file3.rb
 40.00    60.00  func2 - file2.rb
 20.00   100.00  func1 - file1.rb
";

        let mut buf: Vec<u8> = Vec::new();
        stats.write(&mut buf).expect("Callgrind write failed");
        let actual = String::from_utf8(buf).expect("summary output not utf8");
        assert_eq!(actual, expected, "Unexpected summary output");
    }

    #[test]
    fn stats_by_line_number() {
        let mut stats = Stats::new();

        stats.add_lineno(&vec![f(1)]);
        stats.add_lineno(&vec![f(3), f(2), f(1)]);
        stats.add_lineno(&vec![f(2), f(1)]);
        stats.add_lineno(&vec![f(3), f(1)]);
        stats.add_lineno(&vec![f(2), f(3), f(1)]);

        let expected = "% self  % total  name
 40.00    60.00  func3 - file3.rb:3
 40.00    60.00  func2 - file2.rb:2
 20.00   100.00  func1 - file1.rb:1
";

        let mut buf: Vec<u8> = Vec::new();
        stats.write(&mut buf).expect("summary write failed");
        let actual = String::from_utf8(buf).expect("summary output not utf8");
        assert_eq!(actual, expected, "Unexpected summary output");
    }
}
