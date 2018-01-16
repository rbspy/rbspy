extern crate log;

extern crate chrono;
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate libc;
extern crate ruby_stacktrace;

#[cfg(target_os = "macos")]
extern crate regex;

use chrono::prelude::*;
use clap::{App, AppSettings, Arg, ArgMatches};
use libc::pid_t;
use failure::Error;
use failure::ResultExt;

use ruby_stacktrace::*;

fn do_main() -> Result<(), Error> {
    env_logger::init().unwrap();

    let matches: ArgMatches<'static> = arg_parser().get_matches();
    let command = matches.value_of("SUBCOMMAND").unwrap();
    let maybe_pid = matches.value_of("pid");
    match command {
        "snapshot" => {
            let pid_string = maybe_pid.ok_or(format_err!("PID is required for snapshot option"))?;
            let pid = pid_string
                .parse()
                .map_err(|_| format_err!("Invalid PID: {}", pid_string))?;
            Ok(snapshot(pid)?)
        }
        "record" => {
            let maybe_cmd = matches.values_of("cmd");
            let maybe_filename = matches.value_of("file");
            let pid: pid_t = match maybe_pid {
                Some(x) => x.parse().map_err(|_| format_err!("Invalid PID: {}", x))?,
                None => {
                    exec_cmd(&mut maybe_cmd.expect("Either PID or command is required to record"))?
                }
            };
            Ok(record(maybe_filename, pid)?)
        }
        x => Err(format_err!(
            "'{}' is not a valid option: try 'snapshot' or 'record'",
            x
        )),
    }
}

fn main() {
    match do_main() {
        Err(x) => {
            println!("Error. Causes: ");
            for c in x.causes() {
                println!("- {}", c);
            }
            println!("{}", x.backtrace());
            std::process::exit(1);
        }
        _ => {}
    }
}

fn record(filename: Option<&str>, pid: pid_t) -> Result<(), Error> {
    // This gets a stack trace and then just prints it out
    // in a format that Brendan Gregg's stackcollapse.pl script understands
    let process_info = user_interface::process_info(pid)?;
    let mut output = open_record_output(filename)?;
    let ruby_current_thread_address_location = process_info.current_thread_addr_location as u64;
    let stack_trace_function = process_info.stack_trace_function;
    loop {
        let trace = stack_trace_function(ruby_current_thread_address_location, process_info.pid);
        match trace {
            Err(copy::MemoryCopyError::ProcessEnded) => return Ok(()),
            Ok(ok_trace) => {
                for t in ok_trace.iter().rev() {
                    write!(output, "{}", t)?;
                    write!(output, ";")?;
                }
                writeln!(output, " {}", 1)?;
            }
            Err(_) => {
                println!("Dropping one stack trace.");
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn snapshot(pid: pid_t) -> Result<(), Error> {
    let process_info = user_interface::process_info(pid)?;
    let stack_trace_function = process_info.stack_trace_function;
    let trace = stack_trace_function(process_info.current_thread_addr_location, process_info.pid)?;
    for x in trace.iter().rev() {
        println!("{}", x);
    }
    Ok(())
}

fn open_record_output(maybe_filename: Option<&str>) -> Result<Box<std::io::Write>, Error> {
    use std::os::unix::prelude::*;
    match maybe_filename {
        Some(filename) => {
            let path = std::path::Path::new(filename);
            println!("Recording data into {:?}...", path);
            Ok(Box::new(std::fs::File::create(path)?))
        }
        None => {
            let home = std::env::var("HOME")?;
            let dt = Utc::now();
            let date = dt.to_rfc3339();
            let dirname = std::path::Path::new(&home)
                .join(".rbspy")
                .join("records")
                .join(date);
            std::fs::create_dir_all(&dirname)
                .context(format!("Error creating directory {:?}", dirname))?;
            let permissions = std::fs::Permissions::from_mode(0o777);
            std::fs::set_permissions(&dirname, permissions.clone());
            std::fs::set_permissions(
                std::path::Path::new(&home).join(".rbspy"),
                permissions.clone(),
            );
            std::fs::set_permissions(
                std::path::Path::new(&home).join(".rbspy").join("record"),
                permissions,
            );
            let path = dirname.join("stacks.txt");
            println!("Recording data into {:?}...", path);
            Ok(Box::new(std::fs::File::create(path)?))
        }
    }
}

fn exec_cmd(args: &mut std::iter::Iterator<Item = &str>) -> Result<pid_t, Error> {
    let arg1 = args.next().unwrap();
    let pid = std::process::Command::new(arg1).args(args).spawn()?.id() as pid_t;
    Ok(pid)
}

fn arg_parser() -> App<'static, 'static> {
    App::new("ruby-stacktrace")
        .version("0.1")
        .setting(AppSettings::TrailingVarArg)
        .about("Sampling profiler for Ruby programs")
        .arg(
            Arg::with_name("SUBCOMMAND")
                .help(
                    "Subcommand you want to run. Options: top, stackcollapse.\n          top \
                   prints a top-like output of what the Ruby process is doing right now\n          \
                   stackcollapse prints out output suitable for piping to stackcollapse.pl \
                   (https://github.com/brendangregg/FlameGraph)",
                )
                .required(true)
                .index(1),
        )
        .arg(
            Arg::from_usage(
                "-f --file=[FILE] 'File to write output to'",
            ).required(false),
        )
        .arg(
            Arg::from_usage(
                "-p --pid=[PID] 'PID of the Ruby process you want to profile'",
            ).required_unless("cmd"),
        )
        .arg(Arg::from_usage("<cmd>... 'commands to run'").required(
            false,
        ))
}

#[test]
fn test_arg_parsing() {
    let parser = arg_parser();
    // let result = parser.get_matches_from(vec!("ruby-stacktrace", "stackcollapse", "-p", "1234"));
    let result = parser.get_matches_from(vec!["ruby-stacktrace", "stackcollapse", "--pid", "1234"]);
    assert!(result.value_of("pid").unwrap() == "1234");
    assert!(result.value_of("SUBCOMMAND").unwrap() == "stackcollapse");

    let parser = arg_parser();
    let result = parser.get_matches_from(vec!["ruby-stacktrace", "--pid", "1234", "stackcollapse"]);
    assert!(result.value_of("pid").unwrap() == "1234");
    assert!(result.value_of("SUBCOMMAND").unwrap() == "stackcollapse");

    let parser = arg_parser();
    let result =
        parser.get_matches_from(vec!["ruby-stacktrace", "stackcollapse", "ruby", "blah.rb"]);
    let mut cmd_values = result.values_of("cmd").unwrap();
    assert!(cmd_values.next().unwrap() == "ruby");
    assert!(cmd_values.next().unwrap() == "blah.rb");
    assert!(result.value_of("SUBCOMMAND").unwrap() == "stackcollapse");
}
