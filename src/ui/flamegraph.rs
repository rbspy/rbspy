use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};

use core::types::StackFrame;

use failure::{Error, ResultExt};
use tempdir;

const FLAMEGRAPH_SCRIPT: &'static [u8] = include_bytes!("../../vendor/flamegraph/flamegraph.pl");

pub struct Stats {
    pub counts: HashMap<Vec<u8>, usize>,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            counts: HashMap::new(),
        }
    }

    pub fn record(&mut self, stack: &Vec<StackFrame>) -> Result<(), io::Error> {
        let mut buf = vec![];
        for t in stack.iter().rev() {
            write!(&mut buf, "{}", t)?;
            write!(&mut buf, ";")?;
        }
        let count = self.counts.entry(buf).or_insert(0);
        *count += 1;
        Ok(())
    }

    pub fn write(&self, w: File) -> Result<(), Error> {
        let tempdir = tempdir::TempDir::new("flamegraph").unwrap();
        let stacks_file = tempdir.path().join("stacks.txt");
        let mut file = File::create(&stacks_file).expect("couldn't create file");
        for (k, v) in self.counts.iter() {
            file.write(&k)?;
            writeln!(file, " {}", v)?;
        }
        write_flamegraph(stacks_file, w)
    }
}

#[cfg(test)]
mod tests {
    use ui::flamegraph::*;

    // Build a test stackframe
    fn f(i: u32) -> StackFrame {
        StackFrame {
            name: format!("func{}", i),
            relative_path: format!("file{}.rb", i),
            absolute_path: None,
            lineno: i,
        }
    }

    fn assert_contains(counts: &HashMap<Vec<u8>, usize>, s: &str, val: usize) {
        assert_eq!(counts.get(&s.to_string().into_bytes()), Some(&val));
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
        assert_contains(counts, "func1 - file1.rb line 1;", 1);
        assert_contains(counts, "func1 - file1.rb line 1;func3 - file3.rb line 3;func2 - file2.rb line 2;", 3);
        assert_contains(counts, "func1 - file1.rb line 1;func2 - file2.rb line 2;", 2);

        Ok(())
    }
}

// We're not running this test on windows right now for two reasons:
//  1) perl isn't installed by the appveyor CI scripts (yet)
//  2) 'tempdir.close().unwrap()' panics with 'the directory is not empty'
#[cfg(not(windows))]
#[test]
fn test_write_flamegraph() {
    let tempdir = tempdir::TempDir::new("flamegraph").unwrap();
    let stacks_file = tempdir.path().join("stacks.txt");
    let mut file = File::create(&stacks_file).expect("couldn't create file");
    for _ in 1..10 {
        file.write(b"a;a;a;a 1").unwrap();
    }
    let target = File::create(tempdir.path().join("graph.svg")).expect("couldn't create file");
    write_flamegraph(stacks_file, target).expect("Couldn't write flamegraph");
    tempdir.close().unwrap();
}

fn write_flamegraph<P: AsRef<Path>>(source: P, target: File) -> Result<(), Error> {
    let mut child = Command::new("perl")
        .arg("-")
        .arg("--inverted") // icicle graphs are easier to read
        .arg("--minwidth").arg("2") // min width 2 pixels saves on disk space
        .arg(source.as_ref())
        .stdin(Stdio::piped()) // pipe in the flamegraph.pl script to stdin
        .stdout(target)
        .spawn()
        .context("Couldn't execute perl")?;
    // TODO(nll): Remove this silliness after non-lexical lifetimes land.
    {
        let stdin = child.stdin.as_mut().expect("failed to write to stdin");
        stdin.write_all(FLAMEGRAPH_SCRIPT)?;
    }
    child.wait()?;
    Ok(())
}
