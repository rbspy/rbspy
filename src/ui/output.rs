#[cfg(test)]
extern crate tempdir;

use failure::{Error, ResultExt};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use ui::callgrind;
use core::initialize::StackFrame;

const FLAMEGRAPH_SCRIPT: &'static [u8] = include_bytes!("../../vendor/flamegraph/flamegraph.pl");

pub trait Outputter {
    fn record(&mut self, file: &mut File, stack: &Vec<StackFrame>) -> Result<(), Error>;
    fn complete(&mut self, path: &Path, file: File) -> Result<(), Error>;
}

// Uses Brendan Gregg's flamegraph.pl script (which we vendor) to visualize stack traces
pub struct Flamegraph;

impl Outputter for Flamegraph {
    fn record(&mut self, file: &mut File, stack: &Vec<StackFrame>) -> Result<(), Error> {
        // This is the input file format that flamegraph.pl expects: 'a; b; c 1'
        for t in stack.iter().rev() {
            write!(file, "{}", t)?;
            write!(file, ";")?;
        }
        writeln!(file, " {}", 1)?;
        Ok(())
    }

    fn complete(&mut self, path: &Path, file: File) -> Result<(), Error> {
        drop(file); // close it!
        write_flamegraph(path).context("Writing flamegraph failed")?;
        Ok(())
    }
}

pub struct Callgrind(pub callgrind::Stats);

impl Outputter for Callgrind {
    fn record(&mut self, _file: &mut File, stack: &Vec<StackFrame>) -> Result<(), Error> {
        self.0.add(stack);
        Ok(())
    }

    fn complete(&mut self, _path: &Path, mut file: File) -> Result<(), Error> {
        self.0.finish();
        self.0.write(&mut file)?;
        Ok(())
    }
}

#[test]
fn test_write_flamegraph() {
    let tempdir = tempdir::TempDir::new("flamegraph").unwrap();
    let stacks_file = tempdir.path().join("stacks.txt");
    let mut file = File::create(&stacks_file).expect("couldn't create file");
    for _ in 1..10 {
        file.write(b"a;a;a;a 1").unwrap();
    }
    write_flamegraph(stacks_file.to_str().unwrap()).expect("Couldn't write flamegraph");
    tempdir.close().unwrap();
}

fn write_flamegraph<P: AsRef<Path>>(stacks_filename: P) -> Result<(), Error> {
    let stacks_filename = stacks_filename.as_ref();
    let svg_filename = stacks_filename.with_extension("svg");
    let output_svg = File::create(&svg_filename)?;
    eprintln!("Writing flamegraph to {}", svg_filename.display());
    let mut child = Command::new("perl")
        .arg("-")
        .arg(stacks_filename)
        .stdin(Stdio::piped())
        .stdout(output_svg)
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
