#![feature(duration_from_micros)]
#[macro_use]
extern crate log;

extern crate regex;
extern crate libc;
extern crate ruby_stacktrace;
extern crate byteorder;
extern crate clap;
extern crate env_logger;
extern crate read_process_memory;
extern crate time;

use clap::{Arg, App, ArgMatches, AppSettings};
use libc::*;
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


fn get_api_version(pid: pid_t) -> String {
    // this exists because sometimes rbenv takes a while to exec the right Ruby binary.
    // we are dumb right now so we just... wait until it seems to work out.
    let mut i = 0;
    loop {
        let version = address_finder::get_api_version(pid);
        if i > 100 {
            break;
        }
        if version.is_ok() {
            return version.unwrap();
        }
        // if it doesn't work, sleep for 1ms and try again
        i += 1;
        thread::sleep(Duration::from_millis(1));
    }
    panic!("Couldn't find ruby version");
}

fn timestamp() -> u64 {
    let timespec = time::get_time();
    // 1459440009.113178
    let mills: u64 = (timespec.sec as u64 * 1000 * 1000)  as u64 + ((timespec.nsec as u64) / 1000  as u64);
    mills
}

fn main() {
    env_logger::init().unwrap();

    let matches = parse_args();
    let command = matches.value_of("SUBCOMMAND").unwrap();
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

    let version = get_api_version(pid);
    debug!("version: {}", version);

    if command.clone() != "top" && command.clone() != "stackcollapse" &&
        command.clone() != "parse"
    {
        println!("COMMAND must be 'top' or 'stackcollapse. Try again!");
        process::exit(1);
    }

    let ruby_current_thread_address_location = address_finder::current_thread_address_location(pid, &version).unwrap();
    let stack_trace_function = stack_trace::get_stack_trace_function(&version);

    if command == "parse" {
        return;
    } else if command == "stackcollapse" {

        // This gets a stack trace and then just prints it out
        // in a format that Brendan Gregg's stackcollapse.pl script understands
        loop {
            let time_millis = timestamp();
            let trace = stack_trace_function(ruby_current_thread_address_location, pid).unwrap_or_else(|_| {
                // TODO: check that it's actually just a "process doesn't exist" error otherwise complain
                process::exit(0);
            });
            user_interface::print_stack_trace(&trace);
            let time_millis_new = timestamp();
            thread::sleep(Duration::from_micros(10000 - (time_millis_new - time_millis)));
        }
    } else {
        // top subcommand!
        // keeps a running histogram of how often we see every method
        // and periodically reports 'self' and 'total' time for each method
        let mut method_stats = HashMap::new();
        let mut method_own_time_stats = HashMap::new();
        let mut j = 0;
        loop {
            j += 1;
            let trace = stack_trace_function(ruby_current_thread_address_location, pid)
                .unwrap_or_else(|_| { process::exit(0); });
            // only count each element in the stack trace once
            // otherwise recursive methods are overcounted
            let mut seen = HashSet::new();
            for item in &trace {
                if !seen.contains(&item.clone()) {
                    let counter = method_stats.entry(item.clone()).or_insert(0);
                    *counter += 1;
                }
                seen.insert(item.clone());
            }
            {
                let counter2 = method_own_time_stats.entry(trace[0].clone()).or_insert(0);
                *counter2 += 1;
            }
            if j % 100 == 0 {
                user_interface::print_method_stats(&method_stats, &method_own_time_stats, 30);
                method_stats = HashMap::new();
                method_own_time_stats = HashMap::new();
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
