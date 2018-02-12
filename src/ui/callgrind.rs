use std::cmp::min;
use std::collections::{BTreeMap, HashMap};
use std::io;

use core::initialize::StackFrame;

#[derive(Debug)]
struct Call {
    count: usize,
    inclusive: usize,
}

#[derive(Debug, Default)]
struct Location {
    exclusive: usize,
    calls: HashMap<StackFrame, Call>,
}

#[derive(Default, Debug)]
struct Locations(HashMap<StackFrame, Location>);

#[derive(Debug)]
struct StackEntry {
    frame: StackFrame,
    exclusive: usize,
    inclusive: usize,
}

#[derive(Default, Debug)]
pub struct Stats {
    // Stored with the root of the callgraph at the start.
    stack: Vec<StackEntry>,
    locations: Locations,
}

impl Locations {
    fn location(&mut self, frame: &StackFrame) -> &mut Location {
        if !self.0.contains_key(frame) {
            let loc = Location {
                ..Default::default()
            };
            self.0.insert(frame.clone(), loc);
        }
        self.0.get_mut(frame).unwrap()
    }

    fn add_exclusive(&mut self, entry: &StackEntry) {
        self.location(&entry.frame).exclusive += entry.exclusive;
    }

    fn add_inclusive(&mut self, parent: &StackFrame, child: &StackEntry) {
        let ploc = self.location(parent);
        let val = ploc.calls.entry(child.frame.clone()).or_insert(Call {
            count: 0,
            inclusive: 0,
        });
        val.count += 1;
        val.inclusive += child.inclusive;
    }
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            ..Default::default()
        }
    }

    pub fn add(&mut self, stack: &Vec<StackFrame>) {
        // We get input with the root of the callgraph at the end. Reverse that!
        let rev: Vec<_> = stack.iter().rev().collect();

        // Skip any common items
        let mut common = 0;
        let max_common = min(self.stack.len(), rev.len());
        while common < max_common && &self.stack[common].frame == rev[common] {
            common += 1;
        }

        // Pop old items
        while self.stack.len() > common {
            let entry = self.stack.pop().unwrap();
            self.locations.add_exclusive(&entry);
            if let Some(parent) = self.stack.last_mut() {
                self.locations.add_inclusive(&parent.frame, &entry);
                parent.inclusive += entry.inclusive;
            }
        }

        // Add new items
        for i in common..rev.len() {
            self.stack.push(StackEntry {
                frame: rev[i].clone(),
                exclusive: 0,
                inclusive: 0,
            })
        }

        // Count the current entry
        if let Some(entry) = self.stack.last_mut() {
            entry.exclusive += 1;
            entry.inclusive += 1;
        }
    }

    pub fn finish(&mut self) {
        self.add(&vec![]);
    }

    pub fn write(&self, w: &mut io::Write) -> io::Result<()> {
        writeln!(w, "# callgrind format")?;
        writeln!(w, "version: 1")?;
        writeln!(w, "creator: rbspy")?;
        writeln!(w, "events: Samples")?;

        // Sort for consistent results
        let sorted: BTreeMap<_, _> = self.locations.0.iter().collect();
        for (frame, loc) in sorted.iter() {
            writeln!(w, "")?;
            writeln!(w, "fl={}", frame.path())?;
            writeln!(w, "fn={}", &frame.name)?;
            writeln!(w, "{} {}", frame.lineno, loc.exclusive)?;
            let csorted: BTreeMap<_, _> = loc.calls.iter().collect();
            for (cframe, call) in csorted.iter() {
                writeln!(w, "cfl={}", cframe.path())?;
                writeln!(w, "cfn={}", &cframe.name)?;
                writeln!(w, "calls={} {}", call.count, cframe.lineno)?;
                writeln!(w, "{} {}", frame.lineno, call.inclusive)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ui::callgrind::*;

    // Build a test stackframe
    fn f(i: u32) -> StackFrame {
        StackFrame {
            name: format!("func{}", i),
            relative_path: format!("file{}.rb", i),
            absolute_path: None,
            lineno: i,
        }
    }

    // A stack frame from the same file as another one
    fn fdup() -> StackFrame {
        StackFrame {
            name: "funcX".to_owned(),
            relative_path: "file1.rb".to_owned(),
            absolute_path: None,
            lineno: 42,
        }
    }

    fn assert_location(stats: &Stats, f: StackFrame, exclusive: usize, children: usize) {
        let loc = stats
            .locations
            .0
            .get(&f)
            .expect(format!("No location for {}", f).as_ref());
        assert_eq!(loc.exclusive, exclusive, "Bad exclusive time for {}", f,);
        assert_eq!(loc.calls.len(), children, "Bad children count for {}", f,);
    }

    fn assert_inclusive(
        stats: &Stats,
        parent: StackFrame,
        child: StackFrame,
        count: usize,
        inclusive: usize,
    ) {
        let ploc = stats
            .locations
            .0
            .get(&parent)
            .expect(format!("No location for {}", parent).as_ref());
        let call = ploc.calls
            .get(&child)
            .expect(format!("No call of {} in {}", child, parent).as_ref());
        assert_eq!(
            call.count,
            count,
            "Bad inclusive count for {} in {}",
            child,
            parent,
        );
        assert_eq!(
            call.inclusive,
            inclusive,
            "Bad inclusive time for {} in {}",
            child,
            parent,
        )
    }

    fn build_test_stats() -> Stats {
        let mut stats = Stats::new();

        stats.add(&vec![f(1)]);
        stats.add(&vec![f(3), f(2), f(1)]);
        stats.add(&vec![f(2), f(1)]);
        stats.add(&vec![f(3), f(1)]);
        stats.add(&vec![f(2), f(1)]);
        stats.add(&vec![f(3), fdup(), f(1)]);
        stats.finish();

        stats
    }

    #[test]
    fn stats_aggregate() {
        let stats = &build_test_stats();
        assert!(
            stats.stack.is_empty(),
            "Stack not empty: {:#?}",
            stats.stack
        );
        let len = stats.locations.0.len();
        assert_eq!(len, 4, "Bad location count");
        assert_location(stats, f(1), 1, 3);
        assert_location(stats, f(2), 2, 1);
        assert_location(stats, f(3), 3, 0);
        assert_location(stats, fdup(), 0, 1);
        assert_inclusive(stats, f(1), f(2), 2, 3);
        assert_inclusive(stats, f(1), f(3), 1, 1);
        assert_inclusive(stats, f(1), fdup(), 1, 1);
        assert_inclusive(stats, f(2), f(3), 1, 1);
        assert_inclusive(stats, fdup(), f(3), 1, 1);
    }

    #[test]
    fn stats_write() {
        let expected = "# callgrind format
version: 1
creator: rbspy
events: Samples

fl=file1.rb
fn=func1
1 1
cfl=file1.rb
cfn=funcX
calls=1 42
1 1
cfl=file2.rb
cfn=func2
calls=2 2
1 3
cfl=file3.rb
cfn=func3
calls=1 3
1 1

fl=file1.rb
fn=funcX
42 0
cfl=file3.rb
cfn=func3
calls=1 3
42 1

fl=file2.rb
fn=func2
2 2
cfl=file3.rb
cfn=func3
calls=1 3
2 1

fl=file3.rb
fn=func3
3 3
";

        let mut buf: Vec<u8> = Vec::new();
        build_test_stats()
            .write(&mut buf)
            .expect("Callgrind write failed");
        let actual = String::from_utf8(buf).expect("Callgrind output not utf8");
        assert_eq!(actual, expected, "Unexpected callgrind output");
    }
}
