extern crate log;

extern crate chrono;
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate libc;
extern crate rbspy;

#[cfg(target_os = "macos")]
extern crate regex;

use chrono::prelude::*;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use libc::pid_t;
use failure::Error;
use failure::ResultExt;

use rbspy::*;

fn do_main() -> Result<(), Error> {
    env_logger::init().unwrap();

    let matches: ArgMatches<'static> = arg_parser().get_matches();
    match matches.subcommand() {
        ("snapshot", Some(sub_m)) => {
            let pid_string = sub_m.value_of("pid").expect("Failed to find PID");
            let pid = pid_string
                .parse()
                .map_err(|_| format_err!("Invalid PID: {}", pid_string))?;
            Ok(snapshot(pid)?)
        }
        ("record", Some(sub_m)) => {
            let maybe_pid = sub_m.value_of("pid");
            let maybe_cmd = sub_m.values_of("cmd");
            let maybe_filename = sub_m.value_of("file");
            let pid: pid_t = match maybe_pid {
                Some(x) => x.parse().map_err(|_| format_err!("Invalid PID: {}", x))?,
                None => {
                    exec_cmd(&mut maybe_cmd.expect("Either PID or command is required to record"))?
                }
            };
            Ok(record(maybe_filename, pid)?)
        }
        _ => panic!("not a valid subcommand"),
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
    App::new("rbspy")
        .about("Sampling profiler for Ruby programs")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("snapshot")
                .about("Snapshot a single stack trace")
                .arg(
                    Arg::from_usage("-p --pid=[PID] 'PID of the Ruby process you want to profile'")
                        .required_unless("cmd"),
                ),
        )
        .subcommand(
            SubCommand::with_name("record")
                .about("Record process")
                .arg(
                    Arg::from_usage("-p --pid=[PID] 'PID of the Ruby process you want to profile'")
                        .required_unless("cmd"),
                )
                .arg(Arg::from_usage("-f --file=[FILE] 'File to write output to'").required(false))
                .arg(Arg::from_usage("<cmd>... 'commands to run'").required(false)),
        )
}

#[test]
fn test_arg_parsing() {
    let parser = arg_parser();
    // let result = parser.get_matches_from(vec!("rbspy", "stackcollapse", "-p", "1234"));
    let result = parser.get_matches_from(vec!["rbspy", "record", "--pid", "1234"]);
    let result = result.subcommand_matches("record").unwrap();
    assert!(result.value_of("pid").unwrap() == "1234");

    let parser = arg_parser();
    let result = parser.get_matches_from(vec!["rbspy", "snapshot", "--pid", "1234"]);
    let result = result.subcommand_matches("snapshot").unwrap();
    assert!(result.value_of("pid").unwrap() == "1234");

    let parser = arg_parser();
    let result = parser.get_matches_from(vec!["rbspy", "record", "ruby", "blah.rb"]);
    let result = result.subcommand_matches("record").unwrap();
    let mut cmd_values = result.values_of("cmd").unwrap();
    assert!(cmd_values.next().unwrap() == "ruby");
    assert!(cmd_values.next().unwrap() == "blah.rb");
}
