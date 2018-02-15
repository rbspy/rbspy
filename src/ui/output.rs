#[cfg(test)]
extern crate tempdir;

use chrono::prelude::*;
use failure::{Error, ResultExt};

use std;
use std::fs::{File, DirBuilder};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ui::callgrind;
use ui::summary;
use core::initialize::StackFrame;

const FLAMEGRAPH_SCRIPT: &'static [u8] = include_bytes!("../../vendor/flamegraph/flamegraph.pl");

pub struct Output {
    pub file: File,
    pub prefix: String,
    pub path: PathBuf,
    outputter: Box<Outputter>,
}

impl Output {
    pub fn new (output_dir: &Path, outputter: Box<Outputter>) -> Result<Output, Error> {

        let filename = random_filename();
        let prefix = output_dir.join(filename).to_string_lossy().to_string();
        let path = format!("{}{}", prefix, outputter.extension());
        let file = File::create(&path).context(format!(
                "Failed to create output file {}",
                &path
                ))?;
        Ok(Output {
            outputter: outputter,
            path: path.into(),
            prefix,
            file
        })
    }

    pub fn record(&mut self, stack: &Vec<StackFrame>) -> Result<(), Error> {
        self.outputter.record(&mut self.file, stack)
    }

    pub fn complete(mut self) -> Result<(), Error> {
        self.outputter.complete(&self.prefix, self.path.as_ref(), self.file)
    }
}

pub trait Outputter {
    // extension of file to pass into `.record`
    fn extension(&self) -> &'static str;
    fn record(&mut self, file: &mut File, stack: &Vec<StackFrame>) -> Result<(), Error>;
    fn complete(&mut self, prefix: &str, path: &Path, file: File) -> Result<(), Error>;
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
    fn complete(&mut self, prefix: &str, path: &Path, file: File) -> Result<(), Error> {
        drop(file); // close it!
        write_flamegraph(prefix, path).context("Writing flamegraph failed")?;
        Ok(())
    }

    fn extension(&self) -> &'static str { ".raw.txt" }
}

pub struct Callgrind(pub callgrind::Stats);

impl Outputter for Callgrind {
    fn record(&mut self, _file: &mut File, stack: &Vec<StackFrame>) -> Result<(), Error> {
        self.0.add(stack);
        Ok(())
    }

    fn complete(&mut self, _: &str, _: &Path, mut file: File) -> Result<(), Error> {
        self.0.finish();
        self.0.write(&mut file)?;
        Ok(())
    }

    fn extension(&self) -> &'static str { ".callgrind.txt" }
}

pub struct Summary(pub summary::Stats);

impl Outputter for Summary {
    fn record(&mut self, _file: &mut File, stack: &Vec<StackFrame>) -> Result<(), Error> {
        self.0.add_function_name(stack);
        Ok(())
    }

    fn complete(&mut self, _: &str, _: &Path, mut file: File) -> Result<(), Error> {
        self.0.write(&mut file)?;
        Ok(())
    }

    fn extension(&self) -> &'static str { ".summary.txt" }
}

pub struct SummaryLine(pub summary::Stats);

impl Outputter for SummaryLine {
    fn record(&mut self, _file: &mut File, stack: &Vec<StackFrame>) -> Result<(), Error> {
        self.0.add_lineno(stack);
        Ok(())
    }

    fn complete(&mut self, _: &str, _: &Path, mut file: File) -> Result<(), Error> {
        self.0.write(&mut file)?;
        Ok(())
    }
    
    fn extension(&self) -> &'static str { ".summary_by_line.txt" }
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

fn write_flamegraph(prefix: &str, stacks_filename: &Path) -> Result<(), Error> {
    let svg_filename = format!("{}.flamegraph.svg", prefix);
    let output_svg = File::create(&svg_filename)?;
    eprintln!("Writing flamegraph to {}", svg_filename);
    let mut child = Command::new("perl")
        .arg("-")
        .arg("--inverted") // icicle graphs are easier to read
        .arg("--minwidth").arg("2") // min width 2 pixels saves on disk space
        .arg(stacks_filename)
        .stdin(Stdio::piped()) // pipe in the flamegraph.pl script to stdin
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

#[test]
fn test_random_filename() {
    assert!(
        random_filename()
            .to_string_lossy()
            .contains("rbspy-")
    );
}

fn random_filename() -> String {
    use rand::{self, Rng};
    let s = rand::thread_rng()
        .gen_ascii_chars()
        .take(10)
        .collect::<String>();
    format!("{}-{}-{}", "rbspy", Utc::now().format("%Y-%m-%d"), s)
}

pub fn create_output_dir(dir: Option<&str>) -> Result<PathBuf, Error> {
    let dirname = match dir {
        Some(d) => d.into(),
        None => Path::new(&std::env::var("HOME")?).join(".cache/rbspy/records"),
    };
    DirBuilder::new().recursive(true).create(&dirname)?;
    Ok(dirname)
}
