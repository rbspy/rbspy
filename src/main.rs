use anyhow::format_err;
use anyhow::{Context, Error, Result};
use chrono::prelude::*;
use clap::{arg, ArgMatches};
use rand::distributions::Alphanumeric;
use rand::Rng;
use rbspy::recorder;
use rbspy::report;
use rbspy::{OutputFormat, Pid};
use std::env;
use std::fs::DirBuilder;
#[cfg(unix)]
use std::os::unix::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The kinds of things we can call `rbspy record` on.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Target {
    Pid { pid: Pid },
    Subprocess { prog: String, args: Vec<String> },
}

/// Subcommand.
#[derive(Clone, PartialEq, PartialOrd, Debug)]
enum SubCmd {
    /// Record `target`, writing output `output`.
    Record {
        target: Target,
        out_path: PathBuf,
        raw_path: PathBuf,
        sample_rate: u32,
        maybe_duration: Option<std::time::Duration>,
        format: OutputFormat,
        no_drop_root: bool,
        with_subprocesses: bool,
        silent: bool,
        flame_min_width: f64,
        lock_process: bool,
        force_version: Option<String>,
    },
    /// Capture and print a stacktrace snapshot of process `pid`.
    Snapshot {
        pid: Pid,
        lock_process: bool,
        force_version: Option<String>,
    },
    Report {
        format: OutputFormat,
        input: PathBuf,
        output: PathBuf,
    },
    Inspect {
        target: Target,
        force_version: Option<String>,
    },
}

/// Top level args type.
#[derive(Clone, PartialEq, PartialOrd, Debug)]
struct Args {
    cmd: SubCmd,
}

fn main() {
    if let Err(x) = do_main() {
        eprintln!(
            "Something went wrong while rbspy was sampling the process. Here's what we know:"
        );
        for c in x.chain() {
            eprintln!("- {}", c);
        }
        std::process::exit(1);
    }
}

fn do_main() -> Result<(), Error> {
    env_logger::init();

    let args = Args::from_args()?;

    #[cfg(target_os = "macos")]
    {
        let root_cmd = match args.cmd {
            SubCmd::Snapshot { .. } => Some("snapshot"),
            SubCmd::Record { .. } => Some("record"),
            _ => None,
        };
        if let Some(root_cmd) = root_cmd {
            if !nix::unistd::Uid::effective().is_root() {
                return Err(
                    format_err!(
                        concat!(
                            "rbspy {} needs to run as root on Mac. Try rerunning with `sudo --preserve-env !!`. ",
                            "If you run `sudo rbspy record ruby your-program.rb`, rbspy will drop privileges when running `ruby your-program.rb`. If you want the Ruby program to run as root, use `rbspy --no-drop-root`.",
                        ),
                        root_cmd
                    )
                );
            }
        }
    }

    match args.cmd {
        SubCmd::Snapshot {
            pid,
            lock_process,
            force_version,
        } => {
            let snap = recorder::snapshot(pid, lock_process, force_version)?;
            println!("{}", snap);
            Ok(())
        }
        SubCmd::Record {
            target,
            out_path,
            raw_path,
            sample_rate,
            maybe_duration,
            format,
            no_drop_root,
            with_subprocesses,
            silent,
            flame_min_width,
            lock_process,
            force_version,
        } => {
            let pid = match target {
                Target::Pid { pid } => pid,
                Target::Subprocess { prog, args } => spawn_subprocess(prog, args, no_drop_root)?,
            };

            let config = recorder::RecordConfig {
                format,
                raw_path: Some(raw_path.clone()),
                out_path: Some(out_path.clone()),
                pid,
                with_subprocesses,
                sample_rate,
                maybe_duration,
                flame_min_width,
                lock_process,
                force_version,
            };

            let recorder = Arc::<recorder::Recorder>::new(recorder::Recorder::new(config));
            let recorder_handler = recorder.clone();
            let recorder_summary = recorder.clone();
            let interrupted = Arc::<AtomicBool>::new(AtomicBool::new(false));
            let interrupted_handler = interrupted.clone();
            let interrupted_summary = interrupted.clone();
            ctrlc::set_handler(move || {
                if interrupted_handler.load(Ordering::Relaxed) {
                    eprintln!("Multiple interrupts received, exiting with haste!");
                    std::process::exit(1);
                }
                eprintln!("Interrupted.");
                interrupted_handler.store(true, Ordering::Relaxed);
                recorder_handler.stop();
            })
            .expect("Error setting Ctrl-C handler");

            eprintln!("rbspy is recording traces. Press Ctrl+C to stop.");

            let summary_thread = std::thread::spawn(move || {
                if silent {
                    return;
                }

                let mut summary_time = Instant::now() + Duration::from_secs(1);
                loop {
                    if interrupted_summary.load(Ordering::Relaxed) {
                        break;
                    }

                    // Print a summary every second
                    if std::time::Instant::now() > summary_time {
                        println!("{}[2J", 27 as char); // clear screen
                        println!("{}[0;0H", 27 as char); // go to 0,0
                        match recorder_summary.write_summary(&mut std::io::stderr()) {
                            Ok(()) => {}
                            Err(e) => {
                                eprintln!("Failed to print summary: {}", e);
                                break;
                            }
                        };
                        summary_time = Instant::now() + Duration::from_secs(1);
                    }

                    std::thread::sleep(Duration::from_millis(250));
                }
            });

            let recording_result = recorder.record();

            interrupted.store(true, Ordering::Relaxed);
            summary_thread.join().expect("couldn't join summary thread");
            eprintln!(
                "{}",
                format!(
                    "Wrote raw data to {}\nWrote formatted output to {}",
                    raw_path.display(),
                    out_path.display()
                )
            );

            recording_result
        }
        SubCmd::Report {
            format,
            input,
            output,
        } => {
            let mut input = std::fs::File::open(input)?;
            if output.display().to_string() == "-" {
                report(format, &mut input, &mut std::io::stdout())
            } else {
                report(format, &mut input, &mut std::fs::File::create(output)?)
            }
        }
        SubCmd::Inspect {
            target,
            force_version,
        } => {
            let pid = match target {
                Target::Pid { pid } => pid,
                Target::Subprocess { prog, args } => spawn_subprocess(prog, args, true)?,
            };
            rbspy::inspect(pid, force_version)
        }
    }
}

fn arg_parser() -> clap::Command {
    clap::Command::new("rbspy")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Sampling profiler for Ruby programs")
        .subcommand_required(true)
        .subcommand(
            clap::Command::new("snapshot")
                .about("Capture a single stack trace from a running Ruby program")
                .arg(
                    arg!(-p --pid <PID> "PID of the Ruby process you want to profile")
                        .value_parser(validate_pid)
                        .required(true)
                )
                .arg(
                    arg!(--nonblocking "Don't pause the ruby process when taking the snapshot. Setting this option will reduce \
                                                    the performance impact of sampling but may produce inaccurate results")
                        .required(false),
                )
                .arg(
                    clap::Arg::new("force-version")
                        .help("Assume that the Ruby version is <VERSION>. This is useful when the Ruby \
                            version is not yet supported by rbspy, e.g. a release candidate")
                        .long("force-version")
                        .required(false)
                )
        )
        .subcommand(
            clap::Command::new("record")
                .about("Continuously capture traces from a Ruby process")
                .arg(
                    arg!(-p --pid <PID> "PID of the Ruby process you want to profile")
                    .value_parser(validate_pid)
                    // It's a bit confusing but this is how to get exactly-one-of behaviour
                    // for `--pid` and `cmd`.
                    .required_unless_present("cmd")
                    .conflicts_with("cmd"),
                )
                .arg(
                    clap::Arg::new("raw-file")
                        .help("File to write raw data to (will be gzipped)")
                        .long("raw-file")
                        .required(false),
                )
                .arg(
                    arg!(-f --file <FILE> "File to write formatted output to")
                        .required(false),
                )
                .arg(
                    arg!(-r --rate <RATE> "Samples per second collected")
                        .value_parser(clap::value_parser!(u32))
                        .required(false)
                        .default_value("99"),
                )
                .arg(
                    clap::Arg::new("no-drop-root")
                        .action(clap::ArgAction::SetTrue)
                        .help("Don't drop root privileges when running a Ruby program as a subprocess")
                        .short('n')
                        .long("no-drop-root")
                        .required(false),
                )
                .arg(
                    arg!(-o --format <FORMAT> "Output format to write")
                        .value_parser(clap::value_parser!(OutputFormat))
                        .ignore_case(true)
                        .required(false)
                        .default_value("flamegraph"),
                )
                .arg(
                    arg!(-d --duration <DURATION> "Number of seconds to record for")
                        .value_parser(clap::value_parser!(u64))
                        .conflicts_with("cmd")
                        .required(false),
                )
                .arg(
                    arg!(-s --subprocesses "Record all subprocesses of the given PID or command")
                        .action(clap::ArgAction::SetTrue)
                        .required(false)
                )
                .arg(
                    arg!(--silent "Don't print the summary profiling data every second")
                        .action(clap::ArgAction::SetTrue)
                        .required(false)
                )
                .arg(
                    clap::Arg::new("flame-min-width")
                        .value_parser(clap::value_parser!(f64))
                        .help("Minimum flame width in %")
                        .long("flame-min-width")
                        .required(false)
                        .default_value("0.1"),
                )
                .arg(
                    arg!(--nonblocking "Don't pause the ruby process when collecting stack samples. Setting this option will reduce \
                                                   the performance impact of sampling but may produce inaccurate results")
                        .action(clap::ArgAction::SetTrue)
                        .required(false),
                )
                .arg(
                    clap::Arg::new("force-version")
                        .help("Assume that the Ruby version is <VERSION>. This is useful when the Ruby \
                            version is not yet supported by rbspy, e.g. a release candidate")
                        .long("force-version")
                        .required(false)
                )
                .arg(arg!(<cmd> ... "command to run").required(false)),
        )
        .subcommand(
            clap::Command::new("report")
                .about("Generate visualization from raw data recorded by `rbspy record`")
                .arg(
                    arg!(-i --input <FILE> "Input raw data to use")
                        .required(true)
                        .value_parser(clap::value_parser!(PathBuf))
                    )
                .arg(
                    arg!(-o --output <FILE> "Output file")
                        .required(false)
                        .default_value("-")
                        .value_parser(clap::value_parser!(PathBuf))
                )
                .arg(
                    arg!(-f --format <FORMAT> "Output format to write")
                        .value_parser(clap::value_parser!(OutputFormat))
                        .ignore_case(true)
                        .required(false)
                        .default_value("flamegraph"),
                )
        )
        .subcommand(
            clap::Command::new("inspect")
                .about("Inspect a Ruby process, finding key memory addresses that are needed for profiling")
                .hide(true)
                .arg(
                    arg!(-p --pid <PID> "PID of the Ruby process you want to inspect")
                    .value_parser(validate_pid)
                    // It's a bit confusing but this is how to get exactly-one-of behaviour
                    // for `--pid` and `cmd`.
                    .required_unless_present("cmd")
                    .conflicts_with("cmd")
                    .required(false),
                )
                .arg(
                    clap::Arg::new("force-version")
                        .help("Assume that the Ruby version is <VERSION>. This is useful when the Ruby \
                            version is not yet supported by rbspy, e.g. a release candidate")
                        .long("force-version")
                        .required(false)
                )
                .arg(arg!(<cmd> ... "command to run").required(false)),
        )
}

/// Check `s` is a positive integer.
// This assumes a process group isn't a sensible thing to snapshot; could be wrong!
fn validate_pid(s: &str) -> Result<Pid, String> {
    let pid: Pid = s
        .parse()
        .map_err(|_| "PID must be an integer".to_string())?;
    if pid <= 0 {
        return Err("PID must be positive".to_string());
    }
    Ok(pid)
}

impl Args {
    /// Converts from clap's matches.
    // TODO(TryFrom): Replace with TryFrom whenever that stabilizes.
    // TODO(maybe): Consider replacing with one of the derive-based arg thingies.
    fn from<'a, I: IntoIterator<Item = String> + 'a>(args: I) -> Result<Args, Error> {
        let matches: ArgMatches = arg_parser().get_matches_from(args);
        let cmd = match matches.subcommand() {
            Some(("snapshot", submatches)) => SubCmd::Snapshot {
                pid: *submatches
                    .get_one::<Pid>("pid")
                    .expect("this shouldn't happen because clap requires a pid"),
                lock_process: !*submatches.get_one::<bool>("nonblocking").unwrap(),
                force_version: match submatches.get_one::<String>("force-version") {
                    Some(version) => Some(version.to_string()),
                    None => None,
                },
            },
            Some(("record", submatches)) => {
                let format: OutputFormat =
                    ArgMatches::get_one::<OutputFormat>(submatches, "format")
                        .unwrap()
                        .clone();

                let raw_path = output_filename(
                    submatches.get_one::<String>("raw-file").map(|x| x.as_str()),
                    "raw.gz",
                )?;
                let out_path = output_filename(
                    submatches.get_one::<String>("file").map(|x| x.as_str()),
                    &format.extension(),
                )?;
                let maybe_duration = match ArgMatches::get_one::<u64>(submatches, "duration") {
                    Some(integer_duration) => {
                        Some(std::time::Duration::from_secs(*integer_duration))
                    }
                    None => None,
                };

                let no_drop_root = *submatches.get_one::<bool>("no-drop-root").unwrap();
                let silent = *submatches.get_one::<bool>("silent").unwrap();
                let with_subprocesses = *submatches.get_one::<bool>("subprocesses").unwrap();
                let nonblocking = *submatches.get_one::<bool>("nonblocking").unwrap();

                let sample_rate = *ArgMatches::get_one::<u32>(submatches, "rate").unwrap();
                let flame_min_width =
                    *ArgMatches::get_one::<f64>(submatches, "flame-min-width").unwrap();
                let force_version =
                    ArgMatches::get_one::<String>(submatches, "force-version").cloned();
                let target = if let Some(pid) = submatches.get_one::<Pid>("pid") {
                    Target::Pid { pid: *pid }
                } else {
                    let mut cmd = submatches
                        .get_many::<String>("cmd")
                        .expect("shouldn't happen");
                    let prog = cmd.next().expect("nope");
                    let args = cmd;
                    Target::Subprocess {
                        prog: prog.to_string(),
                        args: args.map(String::from).collect(),
                    }
                };
                SubCmd::Record {
                    target,
                    out_path,
                    raw_path,
                    sample_rate,
                    maybe_duration,
                    format,
                    no_drop_root,
                    with_subprocesses,
                    silent,
                    flame_min_width,
                    lock_process: !nonblocking,
                    force_version,
                }
            }
            Some(("report", submatches)) => {
                let format = ArgMatches::get_one::<OutputFormat>(submatches, "format").cloned();
                let input = ArgMatches::get_one::<PathBuf>(submatches, "input").cloned();
                let output = ArgMatches::get_one::<PathBuf>(submatches, "output").cloned();
                SubCmd::Report {
                    format: format.unwrap(),
                    input: input.unwrap(),
                    output: output.unwrap(),
                }
            }
            Some(("inspect", submatches)) => {
                let force_version =
                    ArgMatches::get_one::<String>(submatches, "force-version").cloned();
                let target = if let Some(pid) = submatches.get_one::<Pid>("pid") {
                    Target::Pid { pid: *pid }
                } else {
                    let mut cmd = submatches
                        .get_many::<String>("cmd")
                        .expect("shouldn't happen");
                    let prog = cmd.next().expect("nope");
                    let args = cmd;
                    Target::Subprocess {
                        prog: prog.to_string(),
                        args: args.map(String::from).collect(),
                    }
                };
                SubCmd::Inspect {
                    target,
                    force_version,
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

fn output_filename(maybe_filename: Option<&str>, extension: &str) -> Result<PathBuf, Error> {
    match maybe_filename {
        Some(filename) => Ok(filename.into()),
        None => {
            let s: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(10)
                .map(char::from)
                .collect();
            let filename = format!("{}-{}.{}", Utc::now().format("%Y-%m-%d"), s, extension);
            let dirs = match directories::ProjectDirs::from("", "", "rbspy") {
                Some(dirs) => dirs,
                None => {
                    return Err(format_err!(
                        "Couldn't find a home directory. You might need to set $HOME."
                    ))
                }
            };
            DirBuilder::new()
                .recursive(true)
                .create(&dirs.cache_dir())?;
            Ok(dirs.cache_dir().join(&filename))
        }
    }
}

fn spawn_subprocess(prog: String, args: Vec<String>, no_drop_root: bool) -> Result<Pid> {
    if cfg!(target_os = "macos") {
        // sleep to prevent freezes (because of High Sierra kernel bug)
        // TODO: figure out how to work around this race in a cleaner way
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let context = format!("spawn subprocess '{}'", prog.clone());

    #[cfg(unix)]
    {
        let uid_str = std::env::var("SUDO_UID");
        if nix::unistd::Uid::effective().is_root() && !no_drop_root && uid_str.is_ok() {
            let uid: u32 = uid_str
                .unwrap()
                .parse::<u32>()
                .context("Failed to parse UID")?;
            eprintln!(
                "Dropping permissions: running Ruby command as user {}",
                std::env::var("SUDO_USER").context("SUDO_USER")?
            );
            Ok(std::process::Command::new(prog)
                .uid(uid)
                .args(args)
                .spawn()
                .context(context)?
                .id() as Pid)
        } else {
            Ok(std::process::Command::new(prog)
                .args(args)
                .spawn()
                .context(context)?
                .id() as Pid)
        }
    }
    #[cfg(windows)]
    {
        let _ = no_drop_root;
        Ok(std::process::Command::new(prog)
            .args(args)
            .spawn()
            .context(context)?
            .id() as Pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(args: &str) -> Vec<String> {
        args.split_whitespace().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_record_arg_parsing() {
        // Workaround to avoid modifying read-only directories, e.g. on Nix
        let d = tempfile::tempdir().unwrap();
        let dirname = d.path().to_str().unwrap();
        std::env::set_var("HOME", dirname);

        match Args::from(make_args("rbspy record --pid 1234")).unwrap() {
            Args {
                cmd:
                    SubCmd::Record {
                        target: Target::Pid { pid: 1234 },
                        ..
                    },
            } => (),
            x => panic!("Unexpected: {:?}", x),
        };

        // test record with subcommand
        match Args::from(make_args("rbspy record ruby blah.rb")).unwrap() {
            Args {
                cmd:
                    SubCmd::Record {
                        target: Target::Subprocess { prog, args },
                        ..
                    },
            } => {
                assert_eq!(prog, "ruby");
                assert_eq!(args, vec!["blah.rb".to_string()]);
            }
            x => panic!("Unexpected: {:?}", x),
        };

        let args = Args::from(make_args(
            "rbspy record --pid 1234 --file foo.txt --raw-file raw.gz",
        ))
        .unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Record {
                    target: Target::Pid { pid: 1234 },
                    out_path: "foo.txt".into(),
                    raw_path: "raw.gz".into(),
                    sample_rate: 99,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
                    force_version: None,
                },
            }
        );

        let args = Args::from(make_args(
            "rbspy record --pid 1234 --file foo.txt --raw-file raw.gz --rate 25",
        ))
        .unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Record {
                    target: Target::Pid { pid: 1234 },
                    out_path: "foo.txt".into(),
                    raw_path: "raw.gz".into(),
                    sample_rate: 25,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
                    force_version: None,
                },
            }
        );

        let args = Args::from(make_args(
            "rbspy record --pid 1234 --file foo.txt --raw-file raw.gz --duration 60",
        ))
        .unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Record {
                    target: Target::Pid { pid: 1234 },
                    out_path: "foo.txt".into(),
                    raw_path: "raw.gz".into(),
                    sample_rate: 99,
                    maybe_duration: Some(std::time::Duration::from_secs(60)),
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
                    force_version: None,
                },
            }
        );

        let args = Args::from(make_args(
            "rbspy record --pid 1234 --raw-file raw.gz --file foo.txt --format callgrind --duration 60",
        )).unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Record {
                    target: Target::Pid { pid: 1234 },
                    out_path: "foo.txt".into(),
                    raw_path: "raw.gz".into(),
                    sample_rate: 99,
                    maybe_duration: Some(std::time::Duration::from_secs(60)),
                    format: OutputFormat::callgrind,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
                    force_version: None,
                },
            }
        );

        let args = Args::from(make_args(
            "rbspy record --pid 1234 --raw-file raw.gz --file foo.txt --no-drop-root",
        ))
        .unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Record {
                    target: Target::Pid { pid: 1234 },
                    out_path: "foo.txt".into(),
                    raw_path: "raw.gz".into(),
                    sample_rate: 99,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: true,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
                    force_version: None,
                },
            }
        );

        let args = Args::from(make_args(
            "rbspy record --pid 1234 --raw-file raw.gz --file foo.txt --subprocesses",
        ))
        .unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Record {
                    target: Target::Pid { pid: 1234 },
                    out_path: "foo.txt".into(),
                    raw_path: "raw.gz".into(),
                    sample_rate: 99,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: true,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
                    force_version: None,
                },
            }
        );

        let args = Args::from(make_args(
            "rbspy record --pid 1234 --raw-file raw.gz --file foo.txt --flame-min-width 0.02",
        ))
        .unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Record {
                    target: Target::Pid { pid: 1234 },
                    out_path: "foo.txt".into(),
                    raw_path: "raw.gz".into(),
                    sample_rate: 99,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.02,
                    lock_process: true,
                    force_version: None,
                },
            }
        );

        let args = Args::from(make_args(
            "rbspy record --pid 1234 --raw-file raw.gz --file foo.txt --nonblocking",
        ))
        .unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Record {
                    target: Target::Pid { pid: 1234 },
                    out_path: "foo.txt".into(),
                    raw_path: "raw.gz".into(),
                    sample_rate: 99,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: false,
                    force_version: None,
                },
            }
        );
    }

    #[test]
    fn test_snapshot_arg_parsing() {
        let args = Args::from(make_args("rbspy snapshot --pid 1234")).unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Snapshot {
                    pid: 1234,
                    lock_process: true,
                    force_version: None,
                },
            }
        );
    }

    #[test]
    fn test_report_arg_parsing() {
        let args = Args::from(make_args("rbspy report --input xyz.raw.gz --output xyz")).unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Report {
                    format: OutputFormat::flamegraph,
                    input: PathBuf::from("xyz.raw.gz"),
                    output: PathBuf::from("xyz"),
                },
            }
        );
    }
}
