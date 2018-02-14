use std::cmp::min;
use std::collections::{BTreeMap, HashMap};
use std::io;

use core::initialize::StackFrame;

// Stats about the relationship between two functions, one of which
// calls the other.
#[derive(Debug)]
struct Call {
    // How many times has the outer function called the inner one?
    count: usize,

    // How many samples were found inside the inner function
    // (including all sub-functions), when called by the outer?
    inclusive: usize,
}

// Stats about a single function.
#[derive(Debug, Default)]
struct Location {
    // How many samples were found inside this function only, not
    // including calls to sub-functions?
    exclusive: usize,

    // Data about the calls from this function to other functions.
    calls: HashMap<StackFrame, Call>,
}

// Stats about all functions found in our samples.
#[derive(Default, Debug)]
struct Locations(HashMap<StackFrame, Location>);

// Information about a function currently on the stack.
#[derive(Debug)]
struct StackEntry {
    frame: StackFrame,

    // How many samples were found inside this call only?
    exclusive: usize,

    // How many samples were found in this call, and sub-calls?
    inclusive: usize,
}

// Tracks statistics about a program being sampled.
#[derive(Default, Debug)]
pub struct Stats {
    // The current stack, along with tracking information.
    // The root function is at element zero.
    stack: Vec<StackEntry>,

    // Overall stats about this program.
    locations: Locations,
}

impl Locations {
    // Get the current stats for a StackFrame. If it's never been seen before,
    // automatically create an empty record and return that.
    fn location(&mut self, frame: &StackFrame) -> &mut Location {
        if !self.0.contains_key(frame) {
            // Never seen this frame before, insert an empty record.
            let loc = Location {
                ..Default::default()
            };
            self.0.insert(frame.clone(), loc);
        }
        self.0.get_mut(frame).unwrap()
    }

    // Add to our stats the exclusive time for a given function.
    fn add_exclusive(&mut self, entry: &StackEntry) {
        self.location(&entry.frame).exclusive += entry.exclusive;
    }

    // Add to our stats info about a single call from a parent to a child
    // function.
    fn add_inclusive(&mut self, parent: &StackFrame, child: &StackEntry) {
        let ploc = self.location(parent);
        // If we've never seen this parent-child relationship, insert an empty
        // record.
        let val = ploc.calls.entry(child.frame.clone()).or_insert(Call {
            count: 0,
            inclusive: 0,
        });

        // Add both the count and the inclusive samples cound.
        val.count += 1;
        val.inclusive += child.inclusive;
    }
}

impl Stats {
    // Create an empty stats tracker.
    pub fn new() -> Stats {
        Stats {
            ..Default::default()
        }
    }

    // Add a single stack sample to this Stats.
    pub fn add(&mut self, stack: &Vec<StackFrame>) {
        // The input sample has the root function at the end. Reverse that!
        let rev: Vec<_> = stack.iter().rev().collect();

        // At this point, the previous stack (self.stack) and the new stack
        // (rev) may have some parts that agree and others that differ:
        //
        // Old stack                      New stack
        // +-------+          ^           +------+
        // | root  |          |           | root |
        // +-------+          |           +------+
        // |   A   |        Common        |   A  |
        // +-------+          |           +------+
        // |   B   |          |           |   B  |
        // +-------+      ^   v    ^      +------+
        // |   C   |      |        |      |   X  |
        // +-------+      |     Only new  +------+
        // |   D   |   Only old    |      |   Y  |
        // +-------+      |        v      +------+
        // |   E   |      |
        // +-------+      v
        //
        // Three sections are important:
        //
        // 1. The common base (root, A,  B)
        // 2. The calls only on the previous stack (C, D, E)
        // 3. The calls only on the new stack (X, Y)

        // 1. Common items we can ignore. Find out how many there are, so we
        // can skip them.
        let mut common = 0;
        let max_common = min(self.stack.len(), rev.len());
        while common < max_common && &self.stack[common].frame == rev[common] {
            common += 1;
        }

        // 2. Items only on the previous stack won't be kept, so we have to
        // integrate them into our stats.
        while self.stack.len() > common {
            // For each entry, pop it from our stored stack, and track its
            // exclusive sample count.
            let entry = self.stack.pop().unwrap();
            self.locations.add_exclusive(&entry);

            if let Some(parent) = self.stack.last_mut() {
                // If a parent is present, also track the inclusive sample count.
                self.locations.add_inclusive(&parent.frame, &entry);

                // Inclusive time of the child is also inclusive time of the parent,
                // so attribute it to the parent. If multiple previous items exist,
                // this will in turn be attributed to the grand-parent, etc.
                parent.inclusive += entry.inclusive;
            }
        }
        // Now our stored stack (self.stack) only includes common items, since we
        // popped all the old ones.

        // 3. Add new stack frames to our stored stack.
        for i in common..rev.len() {
            self.stack.push(StackEntry {
                frame: rev[i].clone(),
                exclusive: 0,
                inclusive: 0,
            })
        }
        // Now our stored stack has the same structure as the stack sample (rev).

        // Finally, we have to actually count samples somewhere! Add them to the
        // last entry.
        //
        // We don't increment the inclusive time of everything on the stack here,
        // it's easier to do the addition in step 2 above.
        if let Some(entry) = self.stack.last_mut() {
            entry.exclusive += 1;
            entry.inclusive += 1;
        }
    }

    // Finish adding samples to this Stats.
    pub fn finish(&mut self) {
        // To handle whatever remains on the stored stack, we can just add
        // an empty stack. This causes us to integrate info for each of those
        // frames--see step 2 in add().
        self.add(&vec![]);
    }

    // Write a callgrind file based on the stats collected.
    // SEe the format docs here: http://kcachegrind.sourceforge.net/html/CallgrindFormat.html
    pub fn write(&self, w: &mut io::Write) -> io::Result<()> {
        // Write a header.
        writeln!(w, "# callgrind format")?;
        writeln!(w, "version: 1")?;
        writeln!(w, "creator: rbspy")?;
        writeln!(w, "events: Samples")?;

        // Write the info for each function.
        // Sort first, for consistent ordering.
        let sorted: BTreeMap<_, _> = self.locations.0.iter().collect();
        for (frame, loc) in sorted.iter() {
            writeln!(w, "")?;
            // Exclusive info, along with filename and function name.
            writeln!(w, "fl={}", frame.path())?;
            writeln!(w, "fn={}", &frame.name)?;
            writeln!(w, "{} {}", frame.lineno, loc.exclusive)?;

            // Inclusive info for each function called by this one.
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

    // Assert that basic stats for a stack frame is as expected.
    fn assert_location(stats: &Stats, f: StackFrame, exclusive: usize, children: usize) {
        let loc = stats
            .locations
            .0
            .get(&f)
            .expect(format!("No location for {}", f).as_ref());
        assert_eq!(loc.exclusive, exclusive, "Bad exclusive time for {}", f,);
        assert_eq!(loc.calls.len(), children, "Bad children count for {}", f,);
    }

    // Assert that the inclusive stats for a parent/child pair is as expected.
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
            call.count, count,
            "Bad inclusive count for {} in {}",
            child, parent,
        );
        assert_eq!(
            call.inclusive, inclusive,
            "Bad inclusive time for {} in {}",
            child, parent,
        )
    }

    // Track some fake stats for testing, into a Stats object.
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

    // Test that we aggregate stats correctly.
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

    // Test that we can write stats correctly.
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
