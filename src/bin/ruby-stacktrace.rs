#[macro_use]
extern crate log;

extern crate clap;
extern crate env_logger;
extern crate failure;
extern crate libc;
extern crate read_process_memory;
#[cfg(target_os = "macos")]
extern crate regex;
extern crate ruby_stacktrace;
extern crate time;
extern crate nix;

use nix::sys::ptrace::*;
use clap::{App, AppSettings, Arg, ArgMatches};
use libc::*;
use failure::Error;
use failure::ResultExt;
use std::process;
use std::time::Duration;
use std::thread;
use std::collections::HashMap;
use std::collections::HashSet;
use std::process::Command;

use ruby_stacktrace::*;

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
                "--pause 'Pause Ruby process before collecting stacktrace'",
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

fn parse_args() -> ArgMatches<'static> {
    arg_parser().get_matches()
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

fn get_api_version(pid: pid_t) -> Result<String, Error> {
    // this exists because sometimes rbenv takes a while to exec the right Ruby binary.
    // we are dumb right now so we just... wait until it seems to work out.
    let mut i = 0;
    loop {
        let version = address_finder::get_api_version(pid);
        if i > 100 || version.is_ok() {
            return Ok(version?);
        }
        // if it doesn't work, sleep for 1ms and try again
        i += 1;
        thread::sleep(Duration::from_millis(1));
    }
}

fn timestamp() -> u64 {
    let timespec = time::get_time();
    let mills: u64 =
        (timespec.sec as u64 * 1000 * 1000) as u64 + ((timespec.nsec as u64) / 1000 as u64);
    mills
}

fn main() {
    match do_main() {
        Err(x) => {
            println!("Error. Causes: ");
            for c in x.causes() {
                println!("- {}", c);
            }
            println!("{}", x.backtrace());
            process::exit(1);
        }
        _ => {}
    }
}

struct PtracePid {
    pid: pid_t,
    attached: bool,
}

impl PtracePid {
    fn attach(&mut self) -> Result<(), Error> {
        nix::sys::ptrace::ptrace(ptrace::PTRACE_ATTACH, nix::unistd::Pid::from_raw(self.pid), 0 as * mut c_void, 0 as * mut c_void)?;
        self.attached = true;
        Ok(())
    }

    fn detach(&mut self) -> Result<(), Error> {
        if self.attached {
            nix::sys::ptrace::ptrace(ptrace::PTRACE_DETACH, nix::unistd::Pid::from_raw(self.pid), 0 as * mut c_void, 0 as * mut c_void)?;
            self.attached = false;
        }
        Ok(())
    }
}

impl Drop for PtracePid {
    fn drop(&mut self) {
        self.detach().expect("detach failed");
    }
}

fn do_main() -> Result<(), Error> {
    env_logger::init().unwrap();

    let matches = parse_args();
    let command = matches.value_of("SUBCOMMAND").unwrap();
    let pause = matches.occurrences_of("pause") == 1;
    let maybe_pid = matches.value_of("pid");
    let pid: pid_t = match maybe_pid {
        Some(x) => x.parse().unwrap(),
        None => {
            let mut args = matches.values_of("cmd").unwrap();
            let arg1 = args.next().unwrap();
            let pid = Command::new(arg1)
                .args(args)
                .spawn()
                .expect("command failed to start")
                .id() as pid_t;
            pid
        }
    };

    let version = get_api_version(pid).context("Couldn't get API version")?;
    debug!("version: {}", version);

    if command.clone() != "top" && command.clone() != "stackcollapse" && command.clone() != "parse"
    {
        println!("COMMAND must be 'top' or 'stackcollapse. Try again!");
        process::exit(1);
    }

    let ruby_current_thread_address_location =
        address_finder::current_thread_address_location(pid, &version)?;
    let stack_trace_function = stack_trace::get_stack_trace_function(&version);

    let mut output: Box<std::io::Write> = match matches.value_of("file") {
        Some(filename) => Box::new(std::fs::File::create(filename)?),
        None => Box::new(std::io::stderr()),
    };
    if command == "parse" {
        unimplemented!("parse command not implemented");
    } else if command == "stackcollapse" {
        // This gets a stack trace and then just prints it out
        // in a format that Brendan Gregg's stackcollapse.pl script understands
        loop {
            {
                let mut ptra = PtracePid{pid: pid, attached: false};
                if pause {
                    ptra.attach()?;
                }
                let trace = stack_trace_function(ruby_current_thread_address_location, pid);
                match trace {
                    Err(copy::MemoryCopyError::ProcessEnded) => return Ok(()),
                    Ok(x) => {
                        user_interface::print_stack_trace(&mut output, &x);
                    }
                    Err(y) => {
                        println!("Dropping one stack trace.");
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    } else {
        return Ok(())
    }
}
