#[macro_use]
extern crate clap;

#[cfg(target_os = "macos")]
use anyhow::format_err;
use anyhow::{Context, Error, Result};
use chrono::prelude::*;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use rand::distributions::Alphanumeric;
use rand::Rng;
use rbspy::report;
use rbspy::sampler;
use rbspy::{OutputFormat, Pid};
use std::env;
use std::fs::DirBuilder;
#[cfg(unix)]
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    },
    /// Capture and print a stacktrace snapshot of process `pid`.
    Snapshot { pid: Pid, lock_process: bool },
    Report {
        format: OutputFormat,
        input: PathBuf,
        output: PathBuf,
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
        SubCmd::Snapshot { pid, lock_process } => {
            let snap = sampler::snapshot(pid, lock_process)?;
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
        } => {
            let pid = match target {
                Target::Pid { pid } => pid,
                Target::Subprocess { prog, args } => {
                    if cfg!(target_os = "macos") {
                        // sleep to prevent freezes (because of High Sierra kernel bug)
                        // TODO: figure out how to work around this race in a cleaner way
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }

                    let context = format!("spawn subprocess '{}'", prog.clone());

                    #[cfg(unix)]
                    {
                        let uid_str = std::env::var("SUDO_UID");
                        if nix::unistd::Uid::effective().is_root()
                            && !no_drop_root
                            && uid_str.is_ok()
                        {
                            let uid: u32 = uid_str
                                .unwrap()
                                .parse::<u32>()
                                .context("Failed to parse UID")?;
                            eprintln!(
                                "Dropping permissions: running Ruby command as user {}",
                                std::env::var("SUDO_USER").context("SUDO_USER")?
                            );
                            Command::new(prog)
                                .uid(uid)
                                .args(args)
                                .spawn()
                                .context(context)?
                                .id() as Pid
                        } else {
                            Command::new(prog).args(args).spawn().context(context)?.id() as Pid
                        }
                    }
                    #[cfg(windows)]
                    {
                        let _ = no_drop_root;
                        Command::new(prog).args(args).spawn().context(context)?.id() as Pid
                    }
                }
            };

            let config = sampler::RecordConfig {
                format,
                raw_path: raw_path.clone(),
                out_path: out_path.clone(),
                pid,
                with_subprocesses,
                silent,
                sample_rate,
                maybe_duration,
                flame_min_width,
                lock_process,
            };

            let output_paths_message = format!("Wrote raw data to {}\nWrote formatted output to {}", raw_path.display(), out_path.display());
            match sampler::record(config) {
                Ok(_) => {
                    eprintln!("{}", output_paths_message);
                    Ok(())
                },
                Err(e) => {
                    eprintln!("{}", output_paths_message);
                    Err(e)
                }
            }
        }
        SubCmd::Report {
            format,
            input,
            output,
        } => report(format, input, output),
    }
}

fn arg_parser() -> App<'static, 'static> {
    App::new("rbspy")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Sampling profiler for Ruby programs")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("snapshot")
                .about("Snapshot a single stack trace")
                .arg(
                    Arg::from_usage("-p --pid=[PID] 'PID of the Ruby process you want to profile'")
                        .validator(validate_pid)
                        .required(true)
                )
                .arg(
                    Arg::from_usage("--nonblocking='Don't pause the ruby process when taking the snapshot. Setting this option will reduce \
                                                    the performance impact of sampling but may produce inaccurate results'"),
                )
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
                .arg(
                    Arg::from_usage("--raw-file=[FILE] 'File to write raw data to (will be gzipped)'")
                        .required(false),
                )
                .arg(
                    Arg::from_usage("-f --file=[FILE] 'File to write formatted output to'")
                        .required(false),
                )
                .arg(
                    Arg::from_usage("-r --rate=[RATE] 'Samples per second collected'")
                        .default_value("100"),
                )
                .arg(
                    Arg::from_usage("--no-drop-root 'Don't drop root privileges when running a Ruby program as a subprocess'")
                        .required(false),
                )
                .arg(
                    Arg::from_usage("--format=[FORMAT] 'Output format to write'")
                        .possible_values(&OutputFormat::variants())
                        .case_insensitive(true)
                        .default_value("flamegraph"),
                )
                .arg(
                    Arg::from_usage(
                        "-d --duration=[DURATION] 'Number of seconds to record for'",
                    ).conflicts_with("cmd")
                        .required(false),
                )
                .arg(
                    Arg::from_usage( "-s --subprocesses='Record all subprocesses of the given PID or command'")
                        .required(false)
                )
                .arg(
                    Arg::from_usage( "--silent='Don't print the summary profiling data every second'")
                        .required(false)
                )
                .arg(
                    Arg::from_usage("--flame-min-width='Minimum flame width in %'")
                        .default_value("0.1"),
                )
                .arg(
                    Arg::from_usage("--nonblocking='Don't pause the ruby process when collecting stack samples. Setting this option will reduce \
                                                   the performance impact of sampling but may produce inaccurate results'"),
                )
                .arg(Arg::from_usage("<cmd>... 'command to run'").required(false)),
        )
        .subcommand(
            SubCommand::with_name("report")
                .about("Generate visualization from raw data recorded by `rbspy record`")
                .arg(Arg::from_usage("-i --input=<FILE> 'Input raw data to use'"))
                .arg(Arg::from_usage("-o --output=<FILE> 'Output file'").default_value("-"))
                .arg(
                    Arg::from_usage("-f --format=[FORMAT] 'Output format to write'")
                        .possible_values(&OutputFormat::variants())
                        .case_insensitive(true)
                        .default_value("flamegraph"),
                )
        )
}

/// Check `s` is a positive integer.
// This assumes a process group isn't a sensible thing to snapshot; could be wrong!
fn validate_pid(s: String) -> Result<(), String> {
    let pid: Pid = s
        .parse()
        .map_err(|_| "PID must be an integer".to_string())?;
    if pid <= 0 {
        return Err("PID must be positive".to_string());
    }
    Ok(())
}

impl Args {
    /// Converts from clap's matches.
    // TODO(TryFrom): Replace with TryFrom whenever that stabilizes.
    // TODO(maybe): Consider replacing with one of the derive-based arg thingies.
    fn from<'a, I: IntoIterator<Item = String> + 'a>(args: I) -> Result<Args, Error> {
        let matches: ArgMatches<'a> = arg_parser().get_matches_from(args);

        fn get_pid(matches: &ArgMatches) -> Option<Pid> {
            if let Some(pid_str) = matches.value_of("pid") {
                Some(
                    pid_str
                        .parse()
                        .expect("this shouldn't happen because clap validated the arg"),
                )
            } else {
                None
            }
        }

        fn get_lock_process(matches: &ArgMatches) -> Option<bool> {
            if let Some(lock_process_str) = matches.value_of("lock_process") {
                Some(
                    lock_process_str
                        .parse()
                        .expect("this shouldn't happen because clap validated the arg"),
                )
            } else {
                None
            }
        }

        let cmd = match matches.subcommand() {
            ("snapshot", Some(submatches)) => SubCmd::Snapshot {
                pid: get_pid(submatches)
                    .expect("this shouldn't happen because clap requires a pid"),
                lock_process: get_lock_process(submatches).unwrap_or_default(),
            },
            ("record", Some(submatches)) => {
                let format = value_t!(submatches, "format", OutputFormat).unwrap();

                #[cfg(unix)]
                let home = &std::env::var("HOME").context("HOME")?;
                #[cfg(windows)]
                let home = &std::env::var("userprofile").context("userprofile")?;

                let raw_path = output_filename(home, submatches.value_of("raw-file"), "raw.gz")?;
                let out_path =
                    output_filename(home, submatches.value_of("file"), &format.extension())?;
                let maybe_duration = match value_t!(submatches, "duration", u64) {
                    Err(_) => None,
                    Ok(integer_duration) => Some(std::time::Duration::from_secs(integer_duration)),
                };

                let no_drop_root = submatches.occurrences_of("no-drop-root") == 1;
                let silent = submatches.is_present("silent");
                let with_subprocesses = submatches.is_present("subprocesses");
                let nonblocking = submatches.is_present("nonblocking");

                let sample_rate = value_t!(submatches, "rate", u32).unwrap();
                let flame_min_width = value_t!(submatches, "flame-min-width", f64).unwrap();
                let target = if let Some(pid) = get_pid(submatches) {
                    Target::Pid { pid }
                } else {
                    let mut cmd = submatches.values_of("cmd").expect("shouldn't happen");
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
                }
            }
            ("report", Some(submatches)) => SubCmd::Report {
                format: value_t!(submatches, "format", OutputFormat).unwrap(),
                input: value_t!(submatches, "input", String).unwrap().into(),
                output: value_t!(submatches, "output", String).unwrap().into(),
            },
            _ => panic!("this shouldn't happen, please report the command you ran!"),
        };

        Ok(Args { cmd })
    }

    fn from_args() -> Result<Args, Error> {
        Args::from(env::args())
    }
}
fn output_filename(
    base_dir: &str,
    maybe_filename: Option<&str>,
    extension: &str,
) -> Result<PathBuf, Error> {
    let path = match maybe_filename {
        Some(filename) => filename.into(),
        None => {
            let s: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(10)
                .map(char::from)
                .collect();
            let filename = format!(
                "{}-{}-{}.{}",
                "rbspy",
                Utc::now().format("%Y-%m-%d"),
                s,
                extension
            );
            let dirname = Path::new(base_dir)
                .join(".cache")
                .join("rbspy")
                .join("records");
            DirBuilder::new().recursive(true).create(&dirname)?;
            dirname.join(&filename)
        }
    };
    Ok(path)
}

#[test]
fn test_output_filename() {
    let d = tempdir::TempDir::new("temp").unwrap();
    let dirname = d.path().to_str().unwrap();
    assert_eq!(
        output_filename("", Some("foo"), "txt").unwrap(),
        Path::new("foo")
    );
    let generated_filename = output_filename(dirname, None, "txt").unwrap();

    let filename_pattern = if cfg!(target_os = "windows") {
        ".cache\\rbspy\\records\\rbspy-"
    } else {
        ".cache/rbspy/records/rbspy-"
    };

    assert!(generated_filename
        .to_string_lossy()
        .contains(filename_pattern));
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
            Args {
                cmd:
                    SubCmd::Record {
                        target: Target::Pid { pid: 1234 },
                        ..
                    },
            } => (),
            x => panic!("Unexpected: {:?}", x),
        };

        // test snapshot
        let args = Args::from(make_args("rbspy snapshot --pid 1234")).unwrap();
        assert_eq!(
            args,
            Args {
                cmd: SubCmd::Snapshot {
                    pid: 1234,
                    lock_process: false
                },
            }
        );

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
                    sample_rate: 100,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
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
                    sample_rate: 100,
                    maybe_duration: Some(std::time::Duration::from_secs(60)),
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
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
                    sample_rate: 100,
                    maybe_duration: Some(std::time::Duration::from_secs(60)),
                    format: OutputFormat::callgrind,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
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
                    sample_rate: 100,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: true,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
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
                    sample_rate: 100,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: true,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: true,
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
                    sample_rate: 100,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.02,
                    lock_process: true,
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
                    sample_rate: 100,
                    maybe_duration: None,
                    format: OutputFormat::flamegraph,
                    no_drop_root: false,
                    with_subprocesses: false,
                    silent: false,
                    flame_min_width: 0.1,
                    lock_process: false,
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
