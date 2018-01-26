#![cfg_attr(rustc_nightly, feature(test))]

#[cfg(test)]
extern crate byteorder;
extern crate chrono;
#[macro_use]
extern crate clap;
extern crate ctrlc;
extern crate elf;
extern crate env_logger;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate libc;
#[macro_use]
extern crate log;
extern crate read_process_memory;
#[cfg(test)]
extern crate rbspy_testdata;
extern crate rand;
#[cfg(target_os = "macos")]
extern crate regex;
extern crate ruby_bindings as bindings;
#[cfg(target_os = "linux")]
#[macro_use]
extern crate scopeguard;
#[cfg(test)]
extern crate tempdir;

use chrono::prelude::*;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use libc::pid_t;
use failure::Error;
use failure::ResultExt;
use std::fs::DirBuilder;
use std::io::Write;
use std::path::{PathBuf, Path};
use std::env;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[cfg(test)]
use tempdir::TempDir;

pub mod proc_maps;
pub mod address_finder;
pub mod initialize;
pub mod copy;
pub mod ruby_version;

const FLAMEGRAPH_SCRIPT: &'static [u8] = include_bytes!("../vendor/flamegraph/flamegraph.pl");
const BILLION: u32 = 1000 * 1000 * 1000; // for nanosleep

/// The kinds of things we can call `rbspy record` on.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Target {
    Pid { pid: pid_t },
    Subprocess {
        prog: String,
        args: Vec<String>,
    },
}
use Target::*;

/// Subcommand.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum SubCmd {
    /// Record `target`, writing output `output`.
    Record { target: Target, out_path: PathBuf, sample_rate: u32, maybe_duration: Option<std::time::Duration>},
    /// Capture and print a stacktrace snapshot of process `pid`.
    Snapshot {pid: pid_t},
}
use SubCmd::*;

/// Top level args type.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct Args {
    cmd: SubCmd,
}


fn do_main() -> Result<(), Error> {
    env_logger::init().unwrap();

    let args = Args::from_args()?;

    match args.cmd {
        Snapshot { pid } => snapshot(pid),
        Record { target, out_path, sample_rate, maybe_duration } => {
            let (pid, spawned) = match target {
                Pid {pid} => (pid, false),
                Subprocess { prog, args } => (Command::new(prog)
                    .args(args)
                    .spawn()?.id() as pid_t, true)
            };

            record(&out_path, pid, sample_rate, maybe_duration, spawned)
        },
    }
}

fn main() {
    match do_main() {
        Err(x) => {
            eprintln!("Error. Causes: ");
            for c in x.causes() {
                eprintln!("- {}", c);
            }
            eprintln!("{}", x.backtrace());
            std::process::exit(1);
        }
        _ => {}
    }
}

fn snapshot(pid: pid_t) -> Result<(), Error> {
    let getter = initialize::initialize(pid)?;
    let trace = getter.get_trace()?;
    for x in trace.iter().rev() {
        println!("{}", x);
    }
    Ok(())
}

fn record(output_filename: &Path, pid: pid_t, sample_rate: u32, maybe_duration: Option<std::time::Duration>, is_subcommand: bool) -> Result<(), Error> {
    // This gets a stack trace and then just prints it out
    // in a format that Brendan Gregg's stackcollapse.pl script understands
    let getter = initialize::initialize(pid)?;

    eprintln!("Recording data to {}", output_filename.display());
    let maybe_stop_time = match maybe_duration {
        None => { eprintln!("Press Ctrl+C to stop"); None },
        Some(duration) => { Some(std::time::Instant::now() + duration) }
    };

    let total = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let total_clone = total.clone();
    let errors_clone = errors.clone();

    if is_subcommand {
        // ignore Ctrl+C, on Ctrl+C the subprocess should just exit and then we'll exit normally
        ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");
    } else {
        let outfile = output_filename.to_owned();
        // set a signal handler so that we can write a flamegraph
        ctrlc::set_handler(move || {
            eprintln!("Interrupted.");
            print_errors(errors_clone.clone(), total_clone.clone());
            write_flamegraph(&outfile).expect("Writing flamegraph failed"); std::process::exit(0);
        }).expect("Error setting Ctrl-C handler");
    }

    let mut output = std::fs::File::create(&output_filename).context(format!("Failed to create output file {}", &output_filename.display()))?;
    loop {
        total.fetch_add(1, Ordering::Relaxed);
        let trace = getter.get_trace();
        match trace {
            Err(copy::MemoryCopyError::ProcessEnded) => {
                print_errors(errors, total);
                write_flamegraph(&output_filename).context("Failed to write flamegraph")?;
                return Ok(())
            },
            Ok(ref ok_trace) => {
                for t in ok_trace.iter().rev() {
                    write!(output, "{}", t)?;
                    write!(output, ";")?;
                }
                writeln!(output, " {}", 1)?;
            }
            Err(x) => {
                errors.fetch_add(1, Ordering::Relaxed);
                let e = errors.fetch_add(0, Ordering::Relaxed) as f64;
                let t = total.fetch_add(0, Ordering::Relaxed) as f64;
                if e > 20.0 && e/t > 0.5 {
                    print_errors(errors, total);
                    return Err(x.into());
                }
            }
        }
        if let Some(stop_time) = maybe_stop_time {
            if std::time::Instant::now() > stop_time {
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::new(0, BILLION/sample_rate));
    }
}

fn print_errors(errors: Arc<AtomicUsize>, total: Arc<AtomicUsize>) {
    let e = errors.fetch_add(0, Ordering::Relaxed);
    let t = total.fetch_add(0, Ordering::Relaxed);
    if e > 0 {
        eprintln!("Dropped {}/{} stack traces because of errors.", e, t);
    }
}

#[test]
fn test_write_flamegraph() {
    let tempdir = TempDir::new("flamegraph").unwrap();
    let stacks_file = tempdir.path().join("stacks.txt");
    let mut file = std::fs::File::create(&stacks_file).expect("couldn't create file");
    for _ in 1..10 { file.write(b"a;a;a;a 1").unwrap(); }
    write_flamegraph(stacks_file.to_str().unwrap()).expect("Couldn't write flamegraph");
    tempdir.close().unwrap();
}

fn write_flamegraph<P: AsRef<Path>>(stacks_filename: P) -> Result<(), Error> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let stacks_filename = stacks_filename.as_ref();
    let svg_filename = stacks_filename.with_extension("svg");
    let output_svg = std::fs::File::create(&svg_filename)?;
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



#[test]
fn test_output_filename() {
    let d = TempDir::new("temp").unwrap();
    let dirname = d.path().to_str().unwrap();
    assert_eq!(output_filename("", Some("foo")).unwrap(), Path::new("foo"));
    let generated_filename = output_filename(dirname, None).unwrap();
    assert!(generated_filename.to_string_lossy().contains(".cache/rbspy/records/rbspy-"));
}

fn output_filename(base_dir: &str, maybe_filename: Option<&str>) -> Result<PathBuf, Error> {
    use rand::{self, Rng};

    let path = match maybe_filename {
        Some(filename) => {
            filename.into()
        }
        None => {
            let s = rand::thread_rng().gen_ascii_chars().take(10).collect::<String>();
            let filename = format!("{}-{}-{}.txt", "rbspy", Utc::now().format("%Y-%m-%d"), s);
            let dirname = Path::new(base_dir).join(".cache/rbspy/records");
            DirBuilder::new()
                .recursive(true)
                .create(&dirname)?;
            dirname.join(&filename)
        }
    };
    Ok(path)
}

/// Check `s` is a positive integer.
// This assumes a process group isn't a sensible thing to snapshot; could be wrong!
fn validate_pid(s: String) -> Result<(), String> {
    let pid: pid_t = s.parse()
        .map_err(|_| "PID must be an integer".to_string())?;
    if pid <= 0 {
        return Err("PID must be positive".to_string());
    }
    Ok(())
}

// Prevent collision for the flamegraph filename
fn validate_filename(s: String) -> Result<(), String> {
    if s.ends_with(".svg") {
        return Err("Filename must not end with .svg".to_string());
    }
    Ok(())
}

fn arg_parser() -> App<'static, 'static> {
    App::new("rbspy")
        .about("Sampling profiler for Ruby programs")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("snapshot")
                .about("Snapshot a single stack trace")
                .arg(Arg::from_usage(
                        "-p --pid=[PID] 'PID of the Ruby process you want to profile'")
                    .validator(validate_pid)
                    .required(true)
                ),
        )
        .subcommand(
            SubCommand::with_name("record")
                .about("Record process")
                .arg(
                    Arg::from_usage(
                        "-p --pid=[PID] 'PID of the Ruby process you want to profile'")
                    .validator(validate_pid)
                    // It's a bit confusing but this is how to get exactly-one-of behaviour
                    // for `--pid` and `cmd`.
                    .required_unless("cmd")
                    .conflicts_with("cmd"),
                    )
                .arg(Arg::from_usage("-f --file=[FILE] 'File to write output to'")
                     .validator(validate_filename)
                     .required(false))
                .arg(Arg::from_usage("-r --rate=[RATE] 'Samples per second collected'")
		     .required(false))
                .arg(Arg::from_usage("-d --duration=[DURATION] 'Length of time before ending data collection'")
                .conflicts_with("cmd")
		     .required(false))
                .arg(Arg::from_usage("<cmd>... 'command to run'")
                     .required(false)),
                     )
}

impl Args {
    /// Converts from clap's matches.
    // TODO(TryFrom): Replace with TryFrom whenever that stabilizes.
    // TODO(maybe): Consider replacing with one of the derive-based arg thingies.
    fn from<'a, I: IntoIterator<Item=String> + 'a>(args: I) -> Result<Args, Error> {
        let matches: ArgMatches<'a> = arg_parser().get_matches_from(args);

        fn get_pid(matches: &ArgMatches) -> Option<pid_t> {
            if let Some(pid_str) = matches.value_of("pid") {
                Some(pid_str.parse()
                     .expect("this shouldn't happen because clap validated the arg"))
            } else {
                None
            }
        }

        let cmd = match matches.subcommand() {
            ("snapshot", Some(submatches)) => Snapshot {
                pid: get_pid(submatches)
                    .expect("this shouldn't happen because clap requires a pid")
            },
            ("record", Some(submatches)) => {
                let out_path = output_filename(&std::env::var("HOME")?, submatches.value_of("file"))?;
                let maybe_duration = match value_t!(submatches, "duration", u64) {
                    Err(_) =>  None ,
                    Ok(integer_duration) =>  Some(std::time::Duration::from_secs(integer_duration))
                };

                let sample_rate = value_t!(submatches, "rate", u32).unwrap_or(100);
                let target = if let Some(pid) = get_pid(submatches) {
                    Pid { pid }
                } else {
                    let mut cmd = submatches.values_of("cmd")
                        .expect("shouldn't happen");
                    let prog = cmd.next().expect("nope");
                    let args = cmd;
                    Subprocess {
                        prog: prog.to_string(),
                        args: args.map(String::from).collect(),
                    }
                };
                Record {
                    target,
                    out_path,
                    sample_rate,
                    maybe_duration
                }
            }
            _ => panic!("this shouldn't happen, please report the command you ran!"),
        };

        Ok(Args { cmd })
    }

    fn from_args() -> Result<Args, Error> {
        Args::from(env::args())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_args(args: &str) -> Vec<String> {
        args.split_whitespace().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_arg_parsing() {
        match Args::from(make_args("rbspy record --pid 1234")).unwrap() {
            Args { cmd: Record { target: Pid { pid: 1234 }, .. } } => (),
            x => panic!("Unexpected: {:?}", x),
        };

        let args = Args::from(make_args("rbspy snapshot --pid 1234")).unwrap();
        assert_eq!(args, Args { cmd: Snapshot { pid: 1234 } });

        match Args::from(make_args("rbspy record ruby blah.rb")).unwrap() {
            Args {
                cmd: Record {
                    target: Subprocess { prog, args },
                    ..
                }
            } => {
                assert_eq!(prog, "ruby");
                assert_eq!(args, vec!["blah.rb".to_string()]);
            },
            x => panic!("Unexpected: {:?}", x),
        };

        let args = Args::from(make_args("rbspy record --pid 1234 --file foo.txt")).unwrap();
        assert_eq!(args, Args {
            cmd: Record {
                target: Pid { pid: 1234 },
                out_path: "foo.txt".into(),
                sample_rate: 100,
                maybe_duration: None
            }
        });

        let args = Args::from(make_args("rbspy record --pid 1234 --file foo.txt --rate 25")).unwrap();
        assert_eq!(args, Args {
            cmd: Record {
                target: Pid { pid: 1234 },
                out_path: "foo.txt".into(),
                sample_rate: 25,
                maybe_duration: None
            }
        });

        let args = Args::from(make_args("rbspy record --pid 1234 --file foo.txt --duration 60")).unwrap();
        assert_eq!(args, Args {
            cmd: Record {
                target: Pid { pid: 1234 },
                out_path: "foo.txt".into(),
                sample_rate: 100,
                maybe_duration: Some(std::time::Duration::from_secs(60))
            }
        });
    }
}
