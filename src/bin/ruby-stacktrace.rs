#[macro_use]
extern crate log;

extern crate regex;
extern crate libc;
extern crate ruby_stacktrace;
extern crate byteorder;
extern crate clap;
extern crate env_logger;
extern crate read_process_memory;

use clap::{Arg, App, ArgMatches};
use libc::*;
use std::process;
use std::time::Duration;
use std::thread;
use std::collections::HashMap;
use std::collections::HashSet;

use ruby_stacktrace::*;

fn parse_args() -> ArgMatches<'static> {
    App::new("ruby-stacktrace")
        .version("0.1")
        .about("Sampling profiler for Ruby programs")
        .arg(Arg::with_name("COMMAND")
            .help("Subcommand you want to run. Options: top, stackcollapse.\n          top \
                   prints a top-like output of what the Ruby process is doing right now\n          \
                   stackcollapse prints out output suitable for piping to stackcollapse.pl \
                   (https://github.com/brendangregg/FlameGraph)")
            .required(true)
            .index(1))
        .arg(Arg::with_name("PID")
            .help("PID of the Ruby process you want to profile")
            .required(true)
            .index(2))
        .get_matches()
}


fn main() {
    env_logger::init().unwrap();

    let matches = parse_args();
    let pid: pid_t = matches.value_of("PID").unwrap().parse().unwrap();
    let command = matches.value_of("COMMAND").unwrap();
    if command.clone() != "top" && command.clone() != "stackcollapse" &&
       command.clone() != "parse" {
        println!("COMMAND must be 'top' or 'stackcollapse. Try again!");
        process::exit(1);
    }

    let ruby_current_thread_address_location: u64 = address_finder::current_thread_address_location(pid).unwrap();
    let stack_trace_function = stack_trace::get_stack_trace_function(pid);

    if command == "parse" {
        return;
    } else if command == "stackcollapse" {

        // This gets a stack trace and then just prints it out
        // in a format that Brendan Gregg's stackcollapse.pl script understands
        loop {
            let trace = stack_trace_function(ruby_current_thread_address_location, pid);
            user_interface::print_stack_trace(&trace);
            thread::sleep(Duration::from_millis(10));
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
            let trace = stack_trace_function(ruby_current_thread_address_location, pid);
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
